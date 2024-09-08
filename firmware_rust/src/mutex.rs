use core::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub use crate::hw::Mutex;
pub use avr_device::interrupt::CriticalSection;

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
    pub fn borrow<'cs>(&'cs self, cs: CriticalSection<'cs>) -> Ref<'cs, T> {
        unsafe {
            global_refcnt_inc();
            Ref::new(NonNull::new_unchecked(self.inner.borrow(cs).get()))
        }
    }

    #[inline]
    pub fn borrow_mut<'cs>(&'cs self, cs: CriticalSection<'cs>) -> RefMut<'cs, T> {
        unsafe {
            global_refcnt_inc_mut();
            RefMut::new(NonNull::new_unchecked(self.inner.borrow(cs).get()))
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
    pub fn replace(&self, cs: CriticalSection<'_>, inner: T) -> T {
        self.inner.borrow(cs).replace(inner)
    }

    #[inline]
    pub fn as_ref<'cs>(&self, cs: CriticalSection<'cs>) -> &'cs T {
        unsafe { &*self.inner.borrow(cs).as_ptr() as _ }
    }
}

impl<T> MutexCell<Option<T>> {
    #[inline]
    pub fn as_ref_unwrap<'cs>(&self, cs: CriticalSection<'cs>) -> &'cs T {
        unwrap_option(self.as_ref(cs).as_ref())
    }
}

impl<T: Copy> MutexCell<T> {
    #[inline]
    pub fn get(&self, cs: CriticalSection<'_>) -> T {
        self.inner.borrow(cs).get()
    }

    #[inline]
    pub fn set(&self, cs: CriticalSection<'_>, inner: T) {
        self.inner.borrow(cs).set(inner);
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

// vim: ts=4 sw=4 expandtab
