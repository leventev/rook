use core::{mem::ManuallyDrop, ops::{Deref, DerefMut}};

use crate::arch::x86_64::{enable_interrupts, interrupts_enabled, disable_interrupts};

pub struct InterruptMutex<T> {
    mutex: spin::Mutex<T>,
}

pub struct InterruptMutexGuard<'a, T> {
    guard: ManuallyDrop<spin::MutexGuard<'a, T>>,
    interrupts_enabled: bool,
}

impl<T> InterruptMutex<T> {
    pub const fn new(val: T) -> InterruptMutex<T> {
        InterruptMutex {
            mutex: spin::Mutex::new(val),
        }
    }

    pub fn lock(&self) -> InterruptMutexGuard<T> {
        let interrupts_enabled = interrupts_enabled();
        if interrupts_enabled {
            disable_interrupts();
        }

        InterruptMutexGuard {
            guard: ManuallyDrop::new(self.mutex.lock()),
            interrupts_enabled,
        }
    }
}

impl<'a, T> Drop for InterruptMutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.guard);
        }

        if self.interrupts_enabled {
            enable_interrupts();
        }
    }
}

impl<'a, T> Deref for InterruptMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, T> DerefMut for InterruptMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}
