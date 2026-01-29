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
