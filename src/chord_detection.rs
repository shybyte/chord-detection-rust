use gromagram::Gromagram;
use rusty_machine::learning::naive_bayes::{self, NaiveBayes};
use rusty_machine::linalg::{Matrix, BaseMatrix};
use rusty_machine::learning::SupModel;
use rusty_machine::prelude::BaseMatrixMut;
use std::fmt::Debug;


pub struct ChordDetector<L> {
    gromagram: Gromagram,
    labels: Vec<L>,
    training_input: Vec<f64>,
    training_labels: Vec<f64>,
    model: NaiveBayes<naive_bayes::Gaussian>,
}

impl<L> ChordDetector<L> where
    L: Clone + Eq + Debug
{
    pub fn new(gromagram: Gromagram, labels: &[L]) -> Self {
        ChordDetector {
            gromagram,
            training_input: Vec::new(),
            training_labels: Vec::new(),
            labels: labels.to_vec(),
            model: NaiveBayes::new(),

        }
    }

    pub fn train(&mut self, wav: &[i16], label: &L) {
        let label_encoding = &self.create_label_encoding(label);

        let mut gromagram = &mut self.gromagram;
        let gromagram_len = gromagram.gromagram.len();

        let step_size = gromagram.props.window_size / 4;
        let frames = wav.windows(gromagram.props.window_size).enumerate().filter(|&(i, _)| i % step_size == 0);

        eprintln!("label = {:?}", label);
        for (_, frame) in frames {
            gromagram.reset();
            gromagram.process_audio_frame(frame);
            gromagram.normalize();

            self.training_input.extend(&gromagram.gromagram);
            self.training_labels.extend_from_slice(&label_encoding);
        }
        eprintln!("End Train");
    }

    pub fn finish_training(&mut self) {
        let input_matrix: Matrix<f64> = self.training_input.chunks(self.gromagram.gromagram.len()).collect();
        let label_matrix: Matrix<f64> = self.training_labels.chunks(self.labels.len()).collect();
        self.model.train(&input_matrix, &label_matrix).unwrap();
    }

    pub fn detect(&mut self, gromagram: &[f64]) -> L {
        let m = Matrix::new(1, gromagram.len(), gromagram);
        let prediction_matrix = self.model.predict(&m).unwrap();
        let label_i = prediction_matrix.data().iter().position(|&x| x > 0.9).unwrap();
        self.labels[label_i].clone()
    }

    fn create_label_encoding(&self, label: &L) -> Vec<f64> {
        let mut v = vec![0.0; self.labels.len()];
        let label_i = self.labels.iter().position(|x| x == label).unwrap();
        v[label_i] = 1.0;
        v
    }
}
