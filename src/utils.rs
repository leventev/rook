pub fn align(n: usize, align_by: usize) -> usize {
    if n % align_by == 0 {
        n
    } else {
        n + (align_by - n % align_by)
    }
}

pub fn div_and_ceil(left: usize, right: usize) -> usize {
    if left % right > 0 {
        left / right + 1
    } else {
        left / right
    }
}

pub fn zero_page(table: *mut u64) {
    for i in 0..4096 / 8 {
        unsafe {
            table.offset(i).write(0);
        }
    }
}
