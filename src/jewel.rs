use std::fmt;

#[cfg(target_arch = "x86")]
use core::arch::x86::*;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

/// Jewel provides a uniform interface for SIMD operations.
///
/// To save space, most operations are modify in place.
pub trait Jewel {
    unsafe fn repeating(val: u32, len: usize) -> Self;
    unsafe fn loadu(ptr: *const u8, len: usize) -> Self;
    unsafe fn slow_loadu(&mut self, idx: usize, ptr: *const u8, len: usize, reverse: bool);
    unsafe fn fast_loadu(&mut self, ptr: *const u8);
    unsafe fn add(&mut self, o: Self);
    unsafe fn adds(&mut self, o: Self);
    unsafe fn sub(&mut self, o: Self);
    unsafe fn min(&mut self, o: Self);
    unsafe fn and(&mut self, o: Self);
    unsafe fn cmpeq(&mut self, o: Self);
    unsafe fn cmpgt(&mut self, o: Self);
    unsafe fn blendv(&mut self, o: Self, mask: Self);
    unsafe fn shift_left_1(&mut self);
    unsafe fn shift_right_1(&mut self);
    unsafe fn extract(&self, i: usize) -> u32;
    unsafe fn insert_last_0(&mut self, val: u32);
    unsafe fn insert_last_1(&mut self, val: u32);
    unsafe fn insert_last_2(&mut self, val: u32);
    unsafe fn insert_last_max(&mut self);
    unsafe fn insert_first(&mut self, val: u32);
    unsafe fn insert_first_max(&mut self);

    // for speed, the count_mismatches functions do not require creating a Jewel vector
    unsafe fn mm_count_mismatches(a_ptr: *const u8, b_ptr: *const u8, len: usize) -> u32;
    unsafe fn count_mismatches(a_ptr: *const u8, b_ptr: *const u8, len: usize) -> u32;
}

/// N x 32 x 8 vector backed with 256-bit AVX2 vectors
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[derive(Clone)]
pub struct AvxNx32x8 {
    len: usize,
    v: Vec<__m256i>
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl Jewel for AvxNx32x8 {
    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn repeating(val: u32, len: usize) -> AvxNx32x8 {
        let v = vec![_mm256_set1_epi8(val as i8); (len >> 5) + if (len & 31) > 0 {1} else {0}];

        AvxNx32x8{
            len: len,
            v: v
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn loadu(ptr: *const u8, len: usize) -> AvxNx32x8 {
        let word_len = len >> 5;
        let word_rem = len & 31;
        let mut v = Vec::with_capacity(word_len + if word_rem > 0 {1} else {0});
        let avx2_ptr = ptr as *const __m256i;

        for i in 0..word_len {
            *v.get_unchecked_mut(i) = _mm256_loadu_si256(avx2_ptr.offset(i as isize));
        }

        if word_rem > 0 {
            let mut arr = [0u8; 32];
            let end_ptr = ptr.offset((word_len << 5) as isize);

            for i in 0..word_rem {
                *arr.get_unchecked_mut(i) = *end_ptr.offset(i as isize);
            }

            *v.get_unchecked_mut(word_len) = _mm256_loadu_si256(arr.as_ptr() as *const __m256i);
        }

        AvxNx32x8{
            v: v,
            len: len
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn slow_loadu(&mut self, idx: usize, ptr: *const u8, len: usize, reverse: bool) {
        if len == 0 {
            return;
        }

        let mut arr = [0u8; 32];
        let arr_ptr = arr.as_mut_ptr() as *mut __m256i;

        for i in 0..len {
            let curr_idx = if reverse {idx - i} else {idx + i};
            let arr_idx = curr_idx & 31;

            if arr_idx == 0 || i == 0 {
                _mm256_storeu_si256(arr_ptr, *self.v.get_unchecked(curr_idx >> 5));
            }

            *arr.get_unchecked_mut(arr_idx) = *ptr.offset(i as isize);

            if arr_idx == 31 || i == len - 1 {
                *self.v.get_unchecked_mut(curr_idx >> 5) = _mm256_loadu_si256(arr_ptr);
            }
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn fast_loadu(&mut self, ptr: *const u8) {
        let avx2_ptr = ptr as *const __m256i;

        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_loadu_si256(avx2_ptr.offset(i as isize));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn add(&mut self, o: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_add_epi8(*self.v.get_unchecked(i), *o.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn adds(&mut self, o: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_adds_epi8(*self.v.get_unchecked(i), *o.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn sub(&mut self, o: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_sub_epi8(*self.v.get_unchecked(i), *o.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn min(&mut self, o: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_min_epi8(*self.v.get_unchecked(i), *o.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn and(&mut self, o: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_and_si256(*self.v.get_unchecked(i), *o.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn cmpeq(&mut self, o: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_cmpeq_epi8(*self.v.get_unchecked(i), *o.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn cmpgt(&mut self, o: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_cmpgt_epi8(*self.v.get_unchecked(i), *o.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn blendv(&mut self, o: AvxNx32x8, mask: AvxNx32x8) {
        for i in 0..self.v.len() {
            *self.v.get_unchecked_mut(i) = _mm256_blendv_epi8(*self.v.get_unchecked(i), *o.v.get_unchecked(i), *mask.v.get_unchecked(i));
        }
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn shift_left_1(&mut self) {
        for i in 0..(self.v.len() - 1) {
            let curr = self.v.get_unchecked(i);
            // permute concatenates the second half of the current vector and the first half of the next vector
            *self.v.get_unchecked_mut(i) = _mm256_alignr_epi8(
                _mm256_permute2x128_si256(*curr, *self.v.get_unchecked(i + 1), 0b00100001i32), *curr, 1i32);
        }

        // last one gets to shift in zeros
        let last = self.v.len() - 1;
        let curr = self.v.get_unchecked(last);
        // permute concatenates the second half of the last vector and a vector of zeros
        *self.v.get_unchecked_mut(last) = _mm256_alignr_epi8(_mm256_permute2x128_si256(*curr, *curr, 0b10000001i32), *curr, 1i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn shift_right_1(&mut self) {
        for i in (1..self.v.len()).rev() {
            let curr = self.v.get_unchecked(i);
            // permute concatenates the second half of the previous vector and the first half of the current vector
            *self.v.get_unchecked_mut(i) = _mm256_alignr_epi8(
                *curr, _mm256_permute2x128_si256(*curr, *self.v.get_unchecked(i - 1), 0b00000011i32), 15i32);
        }

        // first one gets to shift in zeros
        let curr = self.v.get_unchecked(0);
        // permute concatenates a vector of zeros and the first half of the first vector
        *self.v.get_unchecked_mut(0) = _mm256_alignr_epi8(*curr, _mm256_permute2x128_si256(*curr, *curr, 0b00001000i32), 15i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn extract(&self, i: usize) -> u32 {
        let idx = i >> 5;
        let j = i & 31;
        let mut arr = [0u8; 32];
        _mm256_storeu_si256(arr.as_mut_ptr() as *mut __m256i, *self.v.get_unchecked(idx));
        *arr.get_unchecked(j) as u32
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn insert_last_0(&mut self, val: u32) {
        let last = self.v.len() - 1;
        *self.v.get_unchecked_mut(last) = _mm256_insert_epi8(*self.v.get_unchecked(last), val as i8, 31i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn insert_last_1(&mut self, val: u32) {
        let last = self.v.len() - 1;
        *self.v.get_unchecked_mut(last) = _mm256_insert_epi8(*self.v.get_unchecked(last), val as i8, 30i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn insert_last_2(&mut self, val: u32) {
        let last = self.v.len() - 1;
        *self.v.get_unchecked_mut(last) = _mm256_insert_epi8(*self.v.get_unchecked(last), val as i8, 29i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn insert_last_max(&mut self) {
        let last = self.v.len() - 1;
        *self.v.get_unchecked_mut(last) = _mm256_insert_epi8(*self.v.get_unchecked(last), i8::max_value(), 31i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn insert_first(&mut self, val: u32) {
        *self.v.get_unchecked_mut(0) = _mm256_insert_epi8(*self.v.get_unchecked(0), val as i8, 0i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn insert_first_max(&mut self) {
        *self.v.get_unchecked_mut(0) = _mm256_insert_epi8(*self.v.get_unchecked(0), i8::max_value(), 0i32);
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn mm_count_mismatches(a_ptr: *const u8, b_ptr: *const u8, len: usize) -> u32 {
        let mut res = 0u32;
        let div_len = (len >> 5) as isize;
        let avx2_a_ptr = a_ptr as *const __m256i;
        let avx2_b_ptr = b_ptr as *const __m256i;

        for i in 0..div_len {
            let a = _mm256_loadu_si256(avx2_a_ptr.offset(i));
            let b = _mm256_loadu_si256(avx2_b_ptr.offset(i));
            let eq = _mm256_cmpeq_epi8(a, b);
            res += _mm256_movemask_epi8(eq).count_ones();
        }

        for i in (div_len << 5)..len as isize {
            res += (*a_ptr.offset(i) == *b_ptr.offset(i)) as u32;
        }

        len as u32 - res
    }

    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn count_mismatches(a_ptr: *const u8, b_ptr: *const u8, len: usize) -> u32 {
        let refresh_len = (len / (255 * 32)) as isize;
        let zeros = _mm256_setzero_si256();
        let mut sad = zeros;
        let avx2_a_ptr = a_ptr as *const __m256i;
        let avx2_b_ptr = b_ptr as *const __m256i;

        for i in 0..refresh_len {
            let mut curr = zeros;

            for j in (i * 255)..((i + 1) * 255) {
                let a = _mm256_loadu_si256(avx2_a_ptr.offset(j));
                let b = _mm256_loadu_si256(avx2_b_ptr.offset(j));
                let eq = _mm256_cmpeq_epi8(a, b);
                curr = _mm256_sub_epi8(curr, eq); // subtract -1 = add 1 when matching
                // counting matches instead of mismatches for speed
            }

            // subtract 0 and sum up 8 bytes at once horizontally into four 64 bit ints
            // accumulate those 64 bit ints
            sad = _mm256_add_epi64(sad, _mm256_sad_epu8(curr, zeros));
        }

        let word_len = (len >> 5) as isize;
        let mut curr = zeros;

        // leftover blocks of 32 bytes
        for i in (refresh_len * 255)..word_len {
            let a = _mm256_loadu_si256(avx2_a_ptr.offset(i));
            let b = _mm256_loadu_si256(avx2_b_ptr.offset(i));
            let eq = _mm256_cmpeq_epi8(a, b);
            curr = _mm256_sub_epi8(curr, eq); // subtract -1 = add 1 when matching
        }

        sad = _mm256_add_epi64(sad, _mm256_sad_epu8(curr, zeros));
        let mut sad_arr = [0u32; 8];
        _mm256_storeu_si256(sad_arr.as_mut_ptr() as *mut __m256i, sad);
        let mut res = *sad_arr.get_unchecked(0) + *sad_arr.get_unchecked(2)
            + *sad_arr.get_unchecked(4) + *sad_arr.get_unchecked(6);

        for i in (word_len << 5)..len as isize {
            res += (*a_ptr.offset(i) == *b_ptr.offset(i)) as u32;
        }

        len as u32 - res
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl fmt::Display for AvxNx32x8 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            #![target_feature(enable = "avx2")]
            write!(f, "[")?;

            let mut arr = [0u8; 32];
            let arr_ptr = arr.as_mut_ptr() as *mut __m256i;

            for i in 0..(self.v.len() - 1) {
                _mm256_storeu_si256(arr_ptr, *self.v.get_unchecked(i));

                for j in 0..32 {
                    write!(f, "{:>3}, ", *arr.get_unchecked(j))?;
                }
            }

            // leftover elements

            _mm256_storeu_si256(arr_ptr, *self.v.get_unchecked(self.v.len() - 1));

            let start = (self.v.len() - 1) << 5;

            for i in 0..(self.len - start) {
                if i == self.len - start - 1 {
                    write!(f, "{:>3}", *arr.get_unchecked(i))?;
                }else{
                    write!(f, "{:>3}, ", *arr.get_unchecked(i))?;
                }
            }

            write!(f, "]")
        }
    }
}
