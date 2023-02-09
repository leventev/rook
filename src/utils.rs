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
