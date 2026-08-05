use rand::Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tensor2D {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<f32>,
}

impl Tensor2D {
    pub fn new(rows: usize, cols: usize, data: Vec<f32>) -> Self {
        assert_eq!(rows * cols, data.len(), "Dimension mismatch in Tensor2D");
        Self { rows, cols, data }
    }

    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0.0; rows * cols],
        }
    }

    pub fn ones(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![1.0; rows * cols],
        }
    }

    pub fn randn(rows: usize, cols: usize, std_dev: f32) -> Self {
        let mut rng = rand::thread_rng();
        let mut data = Vec::with_capacity(rows * cols);
        for _ in 0..(rows * cols) {

            let u1: f32 = rng.gen_range(1e-7..1.0);
            let u2: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let z = (-2.0 * u1.ln()).sqrt() * u2.cos();
            data.push(z * std_dev);
        }
        Self { rows, cols, data }
    }

    #[inline(always)]
    pub fn get(&self, r: usize, c: usize) -> f32 {
        self.data[r * self.cols + c]
    }

    #[inline(always)]
    pub fn set(&mut self, r: usize, c: usize, val: f32) {
        self.data[r * self.cols + c] = val;
    }

    #[inline(always)]
    pub fn row(&self, r: usize) -> &[f32] {
        let start = r * self.cols;
        &self.data[start..start + self.cols]
    }

    #[inline(always)]
    pub fn row_mut(&mut self, r: usize) -> &mut [f32] {
        let start = r * self.cols;
        &mut self.data[start..start + self.cols]
    }

    pub fn matmul(&self, other: &Tensor2D) -> Tensor2D {
        assert_eq!(self.cols, other.rows, "Cannot multiply: inner dimensions do not match");
        let m = self.rows;
        let k = self.cols;
        let n = other.cols;

        let mut c_data = vec![0.0; m * n];

        c_data
            .par_chunks_mut(n)
            .enumerate()
            .for_each(|(i, row_slice)| {
                let a_row = &self.data[i * k..(i + 1) * k];
                for p in 0..k {
                    let a_val = a_row[p];
                    if a_val != 0.0 {
                        let b_row = &other.data[p * n..(p + 1) * n];
                        for j in 0..n {
                            row_slice[j] += a_val * b_row[j];
                        }
                    }
                }
            });

        Tensor2D::new(m, n, c_data)
    }

    pub fn matmul_transposed_b(&self, other: &Tensor2D) -> Tensor2D {
        assert_eq!(self.cols, other.cols, "Cannot multiply: column dimensions do not match for transposed B");
        let m = self.rows;
        let k = self.cols;
        let n = other.rows;

        let mut c_data = vec![0.0; m * n];

        c_data
            .par_chunks_mut(n)
            .enumerate()
            .for_each(|(i, row_slice)| {
                let a_row = &self.data[i * k..(i + 1) * k];
                for j in 0..n {
                    let b_row = &other.data[j * k..(j + 1) * k];
                    let mut sum = 0.0;
                    for p in 0..k {
                        sum += a_row[p] * b_row[p];
                    }
                    row_slice[j] = sum;
                }
            });

        Tensor2D::new(m, n, c_data)
    }

    pub fn add(&self, other: &Tensor2D) -> Tensor2D {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.cols, other.cols);
        let mut out = self.clone();
        out.data.par_iter_mut().zip(&other.data).for_each(|(a, b)| *a += *b);
        out
    }

    pub fn add_assign(&mut self, other: &Tensor2D) {
        assert_eq!(self.rows, other.rows);
        assert_eq!(self.cols, other.cols);
        self.data.par_iter_mut().zip(&other.data).for_each(|(a, b)| *a += *b);
    }

    pub fn scale(&mut self, factor: f32) {
        self.data.par_iter_mut().for_each(|x| *x *= factor);
    }

    pub fn rms_norm(&self, weight: &Tensor2D, eps: f32) -> Tensor2D {
        let m = self.rows;
        let d = self.cols;
        let mut out_data = vec![0.0; m * d];

        out_data
            .par_chunks_mut(d)
            .enumerate()
            .for_each(|(i, row_slice)| {
                let in_row = &self.data[i * d..(i + 1) * d];
                let mut sum_sq = 0.0;
                for &val in in_row {
                    sum_sq += val * val;
                }
                let variance = sum_sq / (d as f32);
                let inv_std = 1.0 / (variance + eps).sqrt();

                for j in 0..d {
                    row_slice[j] = in_row[j] * inv_std * weight.data[j];
                }
            });

        Tensor2D::new(m, d, out_data)
    }

    pub fn apply_rope(&mut self, head_dim: usize, base: f32) {
        let seq_len = self.rows;
        let total_dim = self.cols;
        let num_heads = total_dim / head_dim;

        for pos in 0..seq_len {
            for h in 0..num_heads {
                let head_offset = pos * total_dim + h * head_dim;
                for i in (0..head_dim).step_by(2) {
                    let freq = 1.0 / (base.powf((i as f32) / (head_dim as f32)));
                    let theta = (pos as f32) * freq;
                    let cos = theta.cos();
                    let sin = theta.sin();

                    let idx1 = head_offset + i;
                    let idx2 = head_offset + i + 1;

                    let x1 = self.data[idx1];
                    let x2 = self.data[idx2];

                    self.data[idx1] = x1 * cos - x2 * sin;
                    self.data[idx2] = x1 * sin + x2 * cos;
                }
            }
        }
    }

    pub fn softmax_rowwise(&mut self, causal: bool) {
        let cols = self.cols;
        self.data.par_chunks_mut(cols).enumerate().for_each(|(i, row)| {
            let mut max_val = f32::NEG_INFINITY;
            let limit = if causal { (i + 1).min(cols) } else { cols };

            for j in 0..limit {
                if row[j] > max_val {
                    max_val = row[j];
                }
            }

            let mut sum_exp = 0.0;
            for j in 0..limit {
                let exp = (row[j] - max_val).exp();
                row[j] = exp;
                sum_exp += exp;
            }

            for j in limit..cols {
                row[j] = 0.0;
            }

            if sum_exp > 0.0 {
                let inv_sum = 1.0 / sum_exp;
                for j in 0..limit {
                    row[j] *= inv_sum;
                }
            }
        });
    }

    pub fn swiglu_forward(
        x: &Tensor2D,
        w1: &Tensor2D,
        w2: &Tensor2D,
        w3: &Tensor2D,
    ) -> Tensor2D {
        let x_w1 = x.matmul(w1);
        let x_w3 = x.matmul(w3);

        let rows = x_w1.rows;
        let hidden = x_w1.cols;
        let mut gated_data = vec![0.0; rows * hidden];

        gated_data
            .par_iter_mut()
            .zip(x_w1.data.par_iter().zip(x_w3.data.par_iter()))
            .for_each(|(out, (&v1, &v3))| {

                let silu = v1 / (1.0 + (-v1).exp());
                *out = silu * v3;
            });

        let gated_tensor = Tensor2D::new(rows, hidden, gated_data);
        gated_tensor.matmul(w2)
    }

    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let mut dot = 0.0;
        let mut norm_a = 0.0;
        let mut norm_b = 0.0;

        for (&va, &vb) in a.iter().zip(b.iter()) {
            dot += va * vb;
            norm_a += va * va;
            norm_b += vb * vb;
        }

        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom > 1e-8 {
            dot / denom
        } else {
            0.0
        }
    }

    pub fn normalize_rows(&mut self) {
        let cols = self.cols;
        self.data.par_chunks_mut(cols).for_each(|row| {
            let mut sum_sq = 0.0;
            for &v in row.iter() {
                sum_sq += v * v;
            }
            let norm = sum_sq.sqrt();
            if norm > 1e-8 {
                let inv_norm = 1.0 / norm;
                for v in row.iter_mut() {
                    *v *= inv_norm;
                }
            }
        });
    }

    pub fn adamw_update(
        param: &mut Tensor2D,
        grad: &Tensor2D,
        m: &mut Tensor2D,
        v: &mut Tensor2D,
        step: usize,
        lr: f32,
        beta1: f32,
        beta2: f32,
        eps: f32,
        weight_decay: f32,
    ) {
        assert_eq!(param.data.len(), grad.data.len());
        let beta1_t = 1.0 - beta1.powi(step as i32);
        let beta2_t = 1.0 - beta2.powi(step as i32);

        param.data
            .par_iter_mut()
            .zip(grad.data.par_iter())
            .zip(m.data.par_iter_mut())
            .zip(v.data.par_iter_mut())
            .for_each(|(((p, &g), m_val), v_val)| {

                *p -= lr * weight_decay * *p;

                *m_val = beta1 * *m_val + (1.0 - beta1) * g;
                *v_val = beta2 * *v_val + (1.0 - beta2) * (g * g);

                let m_hat = *m_val / beta1_t;
                let v_hat = *v_val / beta2_t;

                *p -= lr * m_hat / (v_hat.sqrt() + eps);
            });
    }
}

