use rand::seq::SliceRandom;
use rand::Rng;

use crate::rand::distr::uniform::{SampleBorrow, SampleUniform};
use crate::traits::*;
use crate::types::*;
use num::{Bounded, NumCast};
use num_traits::{WrappingAdd, WrappingSub};

use crate::lain_derive::NewFuzzed;

use std::ops::{Add, BitXor, Div, Mul, Sub};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

// set these to 0 to disable
pub const CHANCE_TO_REPEAT_ARRAY_VALUE: f64 = 0.05;
#[cfg(feature = "pick_invalid_enum")]
pub const CHANCE_TO_PICK_INVALID_ENUM: f64 = 0.01;
#[cfg(not(feature = "pick_invalid_enum"))]
pub const CHANCE_TO_PICK_INVALID_ENUM: f64 = 0.0;
#[cfg(feature = "ignore_min_max")]
pub const CHANCE_TO_IGNORE_MIN_MAX: f64 = 0.01;
#[cfg(not(feature = "ignore_min_max"))]
pub const CHANCE_TO_IGNORE_MIN_MAX: f64 = 0.0;

#[repr(u8)]
#[derive(Debug, Copy, Clone, NewFuzzed)]
enum MutatorOperation {
    BitFlip,

    Flip,

    Arithmetic,
}

#[derive(Clone, Debug, Default)]
struct MutatorFlags {
    field_count: Option<usize>,
    all_chances_succeed: bool,
}

/// Represents the state of the current corpus item being fuzzed.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[derive(Default)]
pub struct CorpusFuzzingState {
    fields_fuzzed: usize,
}

impl CorpusFuzzingState {
    pub fn reset(&mut self) {
        self.fields_fuzzed = 0;
    }
}

/// Object which provides helper routines for mutating data structures and RNG management.
#[derive(Debug)]
pub struct Mutator<R: Rng> {
    pub rng: R,
    flags: MutatorFlags,
    corpus_state: CorpusFuzzingState,
}

impl<R: Rng> Mutator<R> {
    pub fn new(rng: R) -> Mutator<R> {
        Mutator {
            rng,
            flags: MutatorFlags::default(),
            corpus_state: CorpusFuzzingState::default(),
        }
    }

    pub fn get_corpus_state(&self) -> CorpusFuzzingState {
        self.corpus_state.clone()
    }

    pub fn set_corpus_state(&mut self, state: CorpusFuzzingState) {
        self.corpus_state = state;
    }

    /// Generates a random choice of the given type
    pub fn gen<T>(&mut self) -> T
    where
        T: NewFuzzed,
        T: 'static,
    {
        T::new_fuzzed(self, None)
    }

    /// Mutates a number after randomly selecting a mutation strategy (see [MutatorOperation] for a list of strategies)
    /// If a min/max is specified then a new number in this range is chosen instead of performing
    /// a bit/arithmetic mutation
    pub fn mutate<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + NumCast
            + Bounded
            + Copy
            + WrappingAdd<Output = T>
            + WrappingSub<Output = T>
            + DangerousNumber<T>
            + std::fmt::Debug,
    {
        // dirty but needs to be done so we can call self.gen_chance_ignore_flags
        if let Some(count) = self.flags.field_count {
            if self.corpus_state.fields_fuzzed == count {
                return;
            }

            self.corpus_state.fields_fuzzed += 1;
        }

        if self.gen_chance(0.01) {
            *num = T::select_dangerous_number(&mut self.rng);
            return;
        }

        let operation = MutatorOperation::new_fuzzed(self, None);

        trace!("Operation selected: {:?}", operation);
        match operation {
            MutatorOperation::BitFlip => self.bit_flip(num),
            MutatorOperation::Flip => self.flip(num),
            MutatorOperation::Arithmetic => self.arithmetic(num),
        }
    }

    /// Flip a single bit in the given number.
    fn bit_flip<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T> + Add<Output = T> + Sub<Output = T> + NumCast + Copy,
    {
        let num_bits = (std::mem::size_of::<T>() * 8) as u8;
        let idx: u8 = self.rng.random_range(0..num_bits);

        trace!("xoring bit {}", idx);

        *num = (*num) ^ num::cast(1u64 << idx).unwrap();
    }

    /// Flip more than 1 bit in this number. This is a flip potentially up to
    /// the max bits in the number
    fn flip<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T> + Add<Output = T> + Sub<Output = T> + NumCast + Copy,
    {
        let num_bits = (std::mem::size_of::<T>() * 8) as u8;
        let bits_to_flip = self.rng.random_range(1..=num_bits) as usize;

        // 64 is chosen here as it's the the max primitive size (in bits) that we support
        // we choose to do this approach over a vec to avoid an allocation
        assert!(num_bits <= 64);
        let mut potential_bit_indices = [0u8; 64];
        for i in 0..num_bits {
            potential_bit_indices[i as usize] = i;
        }

        trace!("flipping {} bits", bits_to_flip);
        let (bit_indices, _) = potential_bit_indices[0..num_bits as usize]
            .partial_shuffle(&mut self.rng, num_bits as usize);

        for idx in bit_indices {
            *num = (*num) ^ num::cast(1u64 << *idx).unwrap()
        }
    }

    /// Perform a simple arithmetic operation on the number (+ or -)
    fn arithmetic<T>(&mut self, num: &mut T)
    where
        T: Add<Output = T>
            + Sub<Output = T>
            + NumCast
            + Copy
            + WrappingAdd<Output = T>
            + WrappingSub<Output = T>,
    {
        let added_num: i64 = self.rng.random_range(1..=0x10);

        if self.rng.random::<bool>() {
            trace!("adding {}", added_num);
            *num = num.wrapping_add(&num::cast(added_num).unwrap());
        } else {
            trace!("subtracting {}", added_num);
            *num = num.wrapping_sub(&num::cast(added_num).unwrap());
        }
    }

    /// Generates a number in the range from [min, max) (**note**: non-inclusive). Panics if min >= max.
    pub fn random_range<T, B1>(&mut self, min: B1, max: B1) -> B1
    where
        T: SampleUniform + std::fmt::Display,
        B1: SampleBorrow<T>
            + std::fmt::Display
            + Add
            + Mul
            + NumCast
            + Sub
            + PartialEq
            + PartialOrd
            + SampleUniform,
    {
        if min >= max {
            panic!("cannot gen number where min ({}) >= max ({})", min, max);
        }
        trace!("generating number between {} and {}", &min, &max);
        let num = self.rng.random_range(min..max);
        trace!("got {}", num);

        num
    }

    /// Generates a number weighted to one end of the interval
    pub fn gen_weighted_range<T, B1>(&mut self, min: B1, max: B1, weighted: Weighted) -> B1
    where
        T: SampleUniform + std::fmt::Display + NumCast,
        B1: SampleBorrow<T>
            + std::fmt::Display
            + std::fmt::Debug
            + Add<Output = B1>
            + Mul<Output = B1>
            + NumCast
            + Sub<Output = B1>
            + PartialEq
            + PartialOrd
            + Copy
            + Div<Output = B1>
            + SampleUniform,
    {
        use crate::rand::distr::{weighted::WeightedIndex, Distribution};

        if weighted == Weighted::None {
            return self.random_range(min, max);
        }

        // weighted numbers are done in a pretty dumb way, but any other way is difficult.
        // the solution is to basically subdivide the range into thirds:
        // 1. The range we're weighted towards with a 70% probability
        // 2. The "midrange" with a 20% probability
        // 3. The opposite end with what should be a 10% probability

        trace!(
            "generating weighted number between {} and {} with weight towards {:?}",
            &min,
            &max,
            weighted
        );

        let range = (max - min) + B1::from(1u8).unwrap();

        if range < B1::from(6u8).unwrap() {
            return self.random_range(min, max);
        }

        let one_third_of_range: B1 = range / B1::from(3u8).unwrap();

        let zero = B1::from(0u8).unwrap();

        let mut slices = [
            ((zero, zero), 0u8),
            ((zero, zero), 0u8),
            ((zero, zero), 0u8),
        ];

        for i in 0..3 {
            let slice_index = B1::from(i).unwrap();
            let min = min + (slice_index * one_third_of_range);
            let max = min + one_third_of_range;

            slices[i as usize] = ((min, max), 0u8);
        }

        // set up the mid range
        // these assignments here represent the weight that each range should get
        (slices[1].1) = 2;

        if weighted == Weighted::Min {
            (slices[0].1) = 7;
            (slices[2].1) = 1;
        } else {
            (slices[0].1) = 1;
            (slices[2].1) = 7;
        }

        // fixup the upper bound which may currently be wrong as a result of integer/floating point math
        // to ensure that we are truly within the user requested min/max
        (slices[2].0).1 = max;

        let dist = WeightedIndex::new(slices.iter().map(|item| item.1)).unwrap();

        let subslice_index = dist.sample(&mut self.rng);
        trace!("got {} subslice index", subslice_index);

        let bounds = slices[subslice_index].0;
        trace!("subslice has bounds {:?}", bounds);

        let num = self.rng.random_range(bounds.0..bounds.1);

        trace!("got {}", num);

        num
    }

    /// Helper function for quitting the recursive mutation early if the target field has already
    /// been mutated.
    pub fn should_early_bail_mutation(&self) -> bool {
        self.flags
            .field_count
            .map(|count| count >= self.corpus_state.fields_fuzzed)
            .unwrap_or(false)
    }

    /// Returns a boolean value indicating whether or not the chance event occurred
    pub fn gen_chance(&mut self, chance_percentage: f64) -> bool {
        if chance_percentage <= 0.0 {
            return false;
        }

        if chance_percentage >= 1.0 {
            return true;
        }

        self.rng.random_bool(chance_percentage)
    }

    /// Client code should call this to signal to the mutator that a new fuzzer iteration is beginning
    /// and that the mutator should reset internal state.
    pub fn random_flags(&mut self) {
        self.flags = MutatorFlags::default();
        self.corpus_state.reset();

        if self.rng.random_bool(0.95) {
            self.flags.field_count = Some(self.random_range(1, 100));
        }
    }

    #[doc(hidden)]
    /// Internal API method that should not be used by clients. This is exposed
    /// publicly for usage in proc macro code
    pub fn increment_fields_fuzzed(&mut self) {
        self.corpus_state.fields_fuzzed += 1;
    }

    pub fn rng_mut(&mut self) -> &mut R {
        &mut self.rng
    }
}
