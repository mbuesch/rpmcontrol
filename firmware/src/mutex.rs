use core::{
    cell::{Cell, UnsafeCell},
    mem::MaybeUninit,
};

pub use crate::hw::Mutex;
pub use avr_device::interrupt::CriticalSection;

macro_rules! define_context {
    ($name:ident) => {
        pub struct $name<'cs>(CriticalSection<'cs>);

        impl<'cs> $name<'cs> {
            /// Create a new context.
            ///
            /// # SAFETY
            ///
            /// This may only be called from the corresponding context.
            /// `MainCtx` may only be constructed from `main()`
            /// and `IrqCtx` may only be constructed from ISRs.
            #[inline(always)]
            pub unsafe fn new() -> Self {
                // SAFETY: This cs is used with the low level PAC primitives.
                //         The IRQ safety is upheld by the context machinery instead.
                //
                //         If a function takes a `MainCtx` argument, it can only be
                //         called from `main()` context. Correspondingly for `IrqCtx`.
                //
                //         At the low level the `MutexCell` and `MutexRefCell` ensure
                //         that they can only being used from the main context.
                //         With this mechanism we can run the main context with IRQs
                //         enabled. There cannot be any concurrency in safe code.
                let cs = unsafe { CriticalSection::new() };
                fence();
                Self(cs)
            }
        }

        impl<'cs> Drop for $name<'cs> {
            #[inline(always)]
            fn drop(&mut self) {
                fence();
            }
        }
    };
}

define_context!(MainCtx);
define_context!(IrqCtx);

impl<'cs> MainCtx<'cs> {
    /// Get the `CriticalSection` that belongs to this context.
    /// In the main context interrupts are enabled.
    /// Therefore, this cs can ONLY be used together with `MutexCell` and `MutexRefCell`.
    #[inline(always)]
    #[allow(dead_code)]
    unsafe fn cs(&self) -> CriticalSection<'cs> {
        self.0
    }
}

impl<'cs> IrqCtx<'cs> {
    /// Get the `CriticalSection` that belongs to this context.
    /// In IRQ context interrupts are disabled.
    /// Therefore, this cs can be used for any critical section work.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn cs(&self) -> CriticalSection<'cs> {
        self.0
    }
}

/// Main context initialization marker.
///
/// This marker does not have a pub constructor.
/// It is only created by [MainCtx].
pub struct MainInitCtx(());

impl MainInitCtx {
    /// Get the `CriticalSection` that belongs to this context.
    /// In initialization context interrupts are disabled.
    /// Therefore, this cs can be used for any critical section work.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn cs<'cs>(&self) -> CriticalSection<'cs> {
        // SAFETY: This can only be called during init with interrupts disabled.
        unsafe { CriticalSection::new() }
    }
}

impl<'cs, 'a> MainCtx<'cs> {
    /// SAFETY: The safety contract of [MainCtx::new] must be upheld.
    #[inline(always)]
    pub unsafe fn new_with_init<F: FnOnce(&'a MainInitCtx)>(f: F) -> Self {
        // SAFETY: We are creating the MainCtx.
        // Therefore, it's safe to construct the MainInitCtx marker.
        f(&MainInitCtx(()));
        // SAFETY: Safety contract of MainCtx::new is upheld.
        unsafe { MainCtx::new() }
    }
}

/// Lazy initialization of static variables.
pub struct LazyMainInit<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> LazyMainInit<T> {
    /// # SAFETY
    ///
    /// It must be ensured that the returned instance is initialized
    /// with a call to [Self::init] during construction of the [MainCtx].
    /// See [MainCtx::new_with_init].
    ///
    /// Using this object in any way before initializing it will
    /// result in Undefined Behavior.
    #[inline(always)]
    pub const unsafe fn uninit() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    #[inline(always)]
    pub fn init(&self, _m: &MainInitCtx, inner: T) -> &T {
        // SAFETY: Initialization is required for the `assume_init` calls.
        unsafe {
            *self.0.get() = MaybeUninit::new(inner);
            (*self.0.get()).assume_init_ref()
        }
    }
}

impl<T> core::ops::Deref for LazyMainInit<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: the `Self::new` safety contract ensures that `Self::init` is called before us.
        unsafe { (*self.0.get()).assume_init_ref() }
    }
}

impl<T> core::ops::DerefMut for LazyMainInit<T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the `Self::new` safety contract ensures that `Self::init` is called before us.
        unsafe { (*self.0.get()).assume_init_mut() }
    }
}

// SAFETY: If T is Send, then we can Send the whole object. The object only contains T state.
unsafe impl<T: Send> Send for LazyMainInit<T> {}

// SAFETY: The `deref` and `deref_mut` functions ensure that they can only be called
//         from `MainCtx` compatible contexts.
unsafe impl<T> Sync for LazyMainInit<T> {}

/// Optimization and reordering fence.
#[inline(always)]
pub fn fence() {
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}

pub struct MutexCell<T> {
    inner: Mutex<Cell<T>>,
}

impl<T> MutexCell<T> {
    #[inline]
    pub const fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(Cell::new(inner)),
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn replace(&self, m: &MainCtx<'_>, inner: T) -> T {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).replace(inner)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn as_ref<'cs>(&self, m: &MainCtx<'cs>) -> &'cs T {
        // SAFETY: The returned reference is bound to the
        //         lifetime of the CriticalSection.
        //         We only use the cs for the main context, where it is allowed to be used.
        unsafe { &*self.inner.borrow(m.cs()).as_ptr() as _ }
    }
}

impl<T: Copy> MutexCell<T> {
    #[inline]
    pub fn get(&self, m: &MainCtx<'_>) -> T {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).get()
    }

    #[inline]
    pub fn set(&self, m: &MainCtx<'_>, inner: T) {
        // SAFETY: We only use the cs for the main context, where it is allowed to be used.
        self.inner.borrow(unsafe { m.cs() }).set(inner);
    }
}

#[repr(transparent)]
pub struct AvrAtomic(UnsafeCell<u8>);

// SAFETY: u8 can be sent between threads.
unsafe impl Send for AvrAtomic {}

// SAFETY: u8 is atomic on AVR.
unsafe impl Sync for AvrAtomic {}

impl AvrAtomic {
    #[inline]
    pub const fn new() -> Self {
        Self(UnsafeCell::new(0))
    }

    #[inline]
    pub fn get(&self) -> u8 {
        fence();
        // SAFETY: u8 load is atomic on AVR.
        let value = unsafe { *self.0.get() };
        fence();
        value
    }

    #[inline]
    pub fn set(&self, value: u8) {
        fence();
        // SAFETY: u8 store is atomic on AVR.
        unsafe { *self.0.get() = value; }
        fence();
    }

    #[inline]
    pub fn get_bool(&self) -> bool {
        self.get() != 0
    }

    #[inline]
    pub fn set_bool(&self, value: bool) {
        self.set(value as _);
    }
}

// vim: ts=4 sw=4 expandtab
