use core::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr::NonNull,
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

            /// Get the `CriticalSection` that belongs to this context.
            #[inline(always)]
            #[allow(dead_code)]
            pub fn cs(&self) -> CriticalSection<'cs> {
                self.0
            }

            /// Convert this to a generic context.
            #[inline(always)]
            pub fn to_any(&self) -> AnyCtx {
                AnyCtx::new()
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

/// Main context initialization marker.
///
/// This marker does not have a pub constructor.
/// It is only created by [MainCtx].
pub struct MainInitCtx(());

impl<'cs, 'a> MainCtx<'cs> {
    /// SAFETY: The safety contract of [MainCtx::new] must be upheld.
    #[inline(always)]
    pub unsafe fn new_with_init<F: FnOnce(&'a MainInitCtx)>(f: F) -> Self {
        // SAFETY: We are creating the MainCtx.
        // Therefore, it's safe to construct the MainInitCtx marker.
        f(&MainInitCtx(()));
        // SAFETY: Safety contract of MainCtx::new is upheld.
        unsafe { Self::new() }
    }
}

pub struct AnyCtx(());

impl AnyCtx {
    /// Create a new generic context.
    #[inline(always)]
    pub fn new() -> Self {
        Self(())
    }

    /// Convert this into a [MainCtx].
    ///
    /// # SAFETY
    ///
    /// You must ensure that either:
    ///
    /// - We actually are running in main context or
    /// - If we are running in interrupt context, then
    ///   all all things done with this MainCtx must be safe w.r.t.
    ///   the interrupted main context.
    ///   e.g. atomic accesses have to be used. etc. etc.
    #[inline(always)]
    pub unsafe fn to_main_ctx<'cs>(&self) -> MainCtx<'cs> {
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
    pub fn init(&self, _m: &MainInitCtx, inner: T) {
        // SAFETY: Initialization is required for the `assume_init` calls.
        unsafe { *self.0.get() = MaybeUninit::new(inner) };
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn deref(&self, _m: &MainCtx) -> &T {
        // SAFETY: the `Self::new` safety contract ensures that `Self::init` is called before us.
        unsafe { (*self.0.get()).assume_init_ref() }
    }

    #[inline(always)]
    #[allow(dead_code)]
    fn deref_mut(&mut self, _m: &MainCtx) -> &mut T {
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

pub struct Ref<'cs, T> {
    inner: NonNull<T>,
    _cs: PhantomData<&'cs T>,
}

impl<'cs, T> Ref<'cs, T> {
    #[inline]
    fn new(inner: NonNull<T>) -> Self {
        Self {
            inner,
            _cs: PhantomData,
        }
    }
}

impl<'cs, T> Deref for Ref<'cs, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl<'cs, T> Drop for Ref<'cs, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { global_refcnt_dec() };
    }
}

pub struct RefMut<'cs, T> {
    inner: NonNull<T>,
    _cs: PhantomData<&'cs mut T>,
}

impl<'cs, T> RefMut<'cs, T> {
    #[inline]
    fn new(inner: NonNull<T>) -> Self {
        Self {
            inner,
            _cs: PhantomData,
        }
    }
}

impl<'cs, T> Deref for RefMut<'cs, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl<'cs, T> DerefMut for RefMut<'cs, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

impl<'cs, T> Drop for RefMut<'cs, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            global_refcnt_dec_mut();
        }
    }
}

static mut GLOBAL_REFCNT: i8 = 0;
const GLOBAL_REFCNT_WRITE: i8 = -1;

#[inline(never)]
unsafe fn global_refcnt_inc() {
    let count = GLOBAL_REFCNT;
    if count < 0 {
        // Already mutably borrowed or too many shared borrows.
        reset_system();
    }
    unsafe {
        GLOBAL_REFCNT = count.wrapping_add(1);
    }
}

#[inline(never)]
unsafe fn global_refcnt_inc_mut() {
    let count = GLOBAL_REFCNT;
    if count != 0 {
        // "MutexRefCell (mut): Already borrowed.
        reset_system();
    }
    unsafe {
        GLOBAL_REFCNT = GLOBAL_REFCNT_WRITE;
    }
}

#[inline(never)]
unsafe fn global_refcnt_dec() {
    unsafe {
        GLOBAL_REFCNT = GLOBAL_REFCNT.wrapping_sub(1);
    }
}

#[inline(always)]
unsafe fn global_refcnt_dec_mut() {
    unsafe {
        GLOBAL_REFCNT = 0;
    }
}

pub struct MutexRefCell<T> {
    inner: Mutex<UnsafeCell<T>>,
}

impl<T> MutexRefCell<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(UnsafeCell::new(value)),
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub fn borrow<'cs>(&'cs self, m: &MainCtx<'cs>) -> Ref<'cs, T> {
        unsafe {
            global_refcnt_inc();
            Ref::new(NonNull::new_unchecked(self.inner.borrow(m.cs()).get()))
        }
    }

    #[inline]
    pub fn borrow_mut<'cs>(&'cs self, m: &MainCtx<'cs>) -> RefMut<'cs, T> {
        unsafe {
            global_refcnt_inc_mut();
            RefMut::new(NonNull::new_unchecked(self.inner.borrow(m.cs()).get()))
        }
    }
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
        self.inner.borrow(m.cs()).replace(inner)
    }

    #[inline]
    #[allow(dead_code)]
    pub fn as_ref<'cs>(&self, m: &MainCtx<'cs>) -> &'cs T {
        unsafe { &*self.inner.borrow(m.cs()).as_ptr() as _ }
    }
}

impl<T: Copy> MutexCell<T> {
    #[inline]
    pub fn get(&self, m: &MainCtx<'_>) -> T {
        self.inner.borrow(m.cs()).get()
    }

    #[inline]
    pub fn set(&self, m: &MainCtx<'_>, inner: T) {
        self.inner.borrow(m.cs()).set(inner);
    }
}

/// Cheaper Option::unwrap() alternative.
///
/// This is cheaper, because it doesn't call into the panic unwind path.
/// Therefore, it does not impose caller-saves overhead onto the calling function.
#[inline(always)]
#[allow(dead_code)]
pub fn unwrap_option<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => reset_system(),
    }
}

/// Cheaper Result::unwrap() alternative.
///
/// This is cheaper, because it doesn't call into the panic unwind path.
/// Therefore, it does not impose caller-saves overhead onto the calling function.
#[inline(always)]
#[allow(dead_code)]
pub fn unwrap_result<T, E>(value: Result<T, E>) -> T {
    match value {
        Ok(value) => value,
        Err(_) => reset_system(),
    }
}

/// Reset the system.
#[inline(always)]
#[allow(clippy::empty_loop)]
pub fn reset_system() -> ! {
    loop {
        // Wait for the watchdog timer to trigger and reset the system.
        // We don't need to disable interrupts here.
        // No interrupt will reset the watchdog timer.
    }
}

#[inline(always)]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    reset_system();
}

// vim: ts=4 sw=4 expandtab
