extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use kzg::{Fr, G1Mul, FFTG1, G1};

use crate::types::fft_settings::MclFFTSettings;
use crate::types::fr::MclFr;
use crate::types::g1::MclG1;

pub fn fft_g1_fast(
    ret: &mut [MclG1],
    data: &[MclG1],
    stride: usize,
    roots: &[MclFr],
    roots_stride: usize,
) {
    let half = ret.len() / 2;
    if half > 0 {
        #[cfg(feature = "parallel")]
        {
            let (lo, hi) = ret.split_at_mut(half);
            rayon::join(
                || fft_g1_fast(lo, data, stride * 2, roots, roots_stride * 2),
                || fft_g1_fast(hi, &data[stride..], stride * 2, roots, roots_stride * 2),
            );
        }

        #[cfg(not(feature = "parallel"))]
        {
            fft_g1_fast(&mut ret[..half], data, stride * 2, roots, roots_stride * 2);
            fft_g1_fast(
                &mut ret[half..],
                &data[stride..],
                stride * 2,
                roots,
                roots_stride * 2,
            );
        }

        for i in 0..half {
            let y_times_root = ret[i + half].mul(&roots[i * roots_stride]);
            ret[i + half] = ret[i].sub(&y_times_root);
            ret[i] = ret[i].add_or_dbl(&y_times_root);
        }
    } else {
        ret[0] = data[0];
    }
}

impl FFTG1<MclG1> for MclFFTSettings {
    fn fft_g1(&self, data: &[MclG1], inverse: bool) -> Result<Vec<MclG1>, String> {
        if data.len() > self.max_width {
            return Err(String::from(
                "Supplied list is longer than the available max width",
            ));
        } else if !data.len().is_power_of_two() {
            return Err(String::from("A list with power-of-two length expected"));
        }

        let stride = self.max_width / data.len();
        let mut ret = vec![MclG1::default(); data.len()];

        let roots = if inverse {
            &self.reverse_roots_of_unity
        } else {
            &self.roots_of_unity
        };

        fft_g1_fast(&mut ret, data, 1, roots, stride);

        if inverse {
            let inv_fr_len = MclFr::from_u64(data.len() as u64).inverse();
            ret[..data.len()]
                .iter_mut()
                .for_each(|f| *f = f.mul(&inv_fr_len));
        }

        Ok(ret)
    }
}

// Used for testing
pub fn fft_g1_slow(
    ret: &mut [MclG1],
    data: &[MclG1],
    stride: usize,
    roots: &[MclFr],
    roots_stride: usize,
) {
    for i in 0..data.len() {
        // Evaluate first member at 1
        ret[i] = data[0].mul(&roots[0]);

        // Evaluate the rest of members using a step of (i * J) % data.len() over the roots
        // This distributes the roots over correct x^n members and saves on multiplication
        for j in 1..data.len() {
            let v = data[j * stride].mul(&roots[((i * j) % data.len()) * roots_stride]);
            ret[i] = ret[i].add_or_dbl(&v);
        }
    }
}
