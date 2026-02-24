#[easy_ext::ext(Float32Ext)]
pub impl f32 {
    /// Truncate to usize.
    fn _trunc(&self) -> usize {
        self.trunc() as usize
    }

    /// Round to usize.
    fn _round(&self) -> usize {
        self.round() as usize
    }
}

#[easy_ext::ext(UsizeExt)]
pub impl usize {
    /// Saturating subtract with lower bound 0.
    fn ssub(&mut self, rhs: usize) -> bool {
        let worked = *self != 0;
        *self = self.saturating_sub(rhs);
        worked
    }

    /// Wrap subtract with wrap around at cap.
    fn wsub(&mut self, rhs: usize, cap: usize) -> bool {
        let worked = *self <= rhs;
        *self = (cap + *self + cap - (rhs % cap)) % cap;
        worked
    }
}
