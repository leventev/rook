use alloc::vec::Vec;

const DEFAULT_SLOT_COUNT: usize = 8;

/// A data structure that is used to allocate and deallocate slots.
/// An allocation finds the first unallocated slot, marks it as unallocated
/// and returns the index.
/// A deallocation simply marks the slot as unallocated without copying the
/// elements after it thus preserving the validity of the allocated slot indexes.
/// An example use case of this would be process ID allocation.
#[derive(Debug, Clone)]
pub struct SlotAllocator<T> {
    /// Inner vector for storing slots
    inner: Vec<Option<T>>,

    /// Number of allocated slots
    allocated_slots: usize,

    /// Number of maximum allocated slots, optional
    max_slots: Option<usize>,
}

impl<T> SlotAllocator<T> {
    /// Creates a new, empty `SlotAllocArray<T>`.
    ///
    /// The inner `Vec<T>` will not allocate until a slot is being allocated.
    ///
    /// The upper limit of the number of slots can be specified.
    pub const fn new(max_slots: Option<usize>) -> SlotAllocator<T> {
        SlotAllocator {
            inner: Vec::new(),
            allocated_slots: 0,
            max_slots,
        }
    }
}

impl<T> SlotAllocator<T> {
    /// Doubles the size of the inner `Vec<T>` until the hint can fit in it
    fn resize_for_hint(&mut self, hint: usize) -> usize {
        let mut size = self.inner.len();

        if size == 0 {
            size = DEFAULT_SLOT_COUNT;
        }

        while size <= hint {
            size *= 2;
        }

        if let Some(max) = self.max_slots {
            size = usize::min(size, max);
        }

        self.inner.resize_with(size, || None);
        hint
    }

    // Doubles the size of the inner `Vec<T>` if all the slots have been allocated
    fn resize_double(&mut self) -> usize {
        let full = self.inner.len() == self.allocated_slots;
        if full {
            let old_len = self.inner.len();

            // if this is the first time we are allocating set the length to be
            // DEFAULT_SLOT_COUNT else double the current length
            let new_len = if old_len == 0 {
                DEFAULT_SLOT_COUNT
            } else {
                old_len * 2
            };

            // if we wanted to use `Vec::resize` we would need to make T: Clone
            self.inner.resize_with(new_len, || None);

            old_len
        } else {
            self.inner.iter().position(Option::is_none).unwrap()
        }
    }

    fn allocate_slot(&mut self, val: T, hint: Option<usize>) -> usize {
        // at this point the slot at `hint` is guaranteed to be unanallocated
        let index = match hint {
            Some(hint) => self.resize_for_hint(hint),
            None => self.resize_double(),
        };

        self.allocated_slots += 1;
        self.inner[index] = Some(val);

        index
    }

    fn deallocate_slot(&mut self, index: usize) {
        if self.is_allocated(index) {
            panic!("invalid slot index: {}", index);
        }

        // TODO: is the value dropped?
        self.allocated_slots -= 1;
        self.inner[index] = None;
    }

    /// Returns the number of allocated slots
    pub fn allocated_slots(&self) -> usize {
        self.allocated_slots
    }

    /// Returns whether `index` is a valid slot index
    pub fn is_valid_index(&self, index: usize) -> bool {
        index < self.inner.len()
    }

    /// Returns whether the slot at `index` is allocated
    pub fn is_allocated(&self, index: usize) -> bool {
        self.inner[index].is_some()
    }

    /// Returns whether the slot at `index` can be allocated
    pub fn can_alloc_at(&self, index: usize) -> bool {
        self.is_valid_index(index) && !self.is_allocated(index)
    }

    /// Returns a shared reference to the inner value at `index`
    pub fn get(&self, index: usize) -> Option<&T> {
        match self.inner.get(index) {
            Some(slot) => slot.as_ref(),
            None => None,
        }
    }

    /// Returns a mutable reference to the inner value at `index`
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        match self.inner.get_mut(index) {
            Some(slot) => slot.as_mut(),
            None => None,
        }
    }

    /// Deallocates all slots
    pub fn clear(&mut self) {
        // TODO: maybe free the memory
        // if we wanted to use `Vec::fill` we would need to make T: Clone
        self.inner.fill_with(|| None);
        self.allocated_slots = 0;
    }

    /// Tries to allocate a slot and moves `val` there. If the maximum number of slots that can be
    /// allocated is reached or the slot at `hint` is already allocated `None` is returned.
    pub fn allocate(&mut self, hint: Option<usize>, val: T) -> Option<usize> {
        // TODO: return Result
        if let Some(max) = self.max_slots {
            if self.allocated_slots >= max {
                return None;
            }

            if let Some(hint) = hint {
                if hint >= max {
                    return None;
                }
            }
        }

        Some(self.allocate_slot(val, hint))
    }

    /// Deallocates a slot at `index`, it panics if the slot at `index` does not exist
    /// or it is unallocated
    pub fn deallocate(&mut self, index: usize) {
        self.deallocate_slot(index);
    }
}
