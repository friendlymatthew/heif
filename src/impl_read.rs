#[macro_export]
macro_rules! impl_read_for_datatype {
    ($name:ident, $type:ty) => {
        fn $name(&mut self) -> Result<$type> {
            let width = std::mem::size_of::<$type>();
            let slice = self.read_slice(width)?;
            let b = <$type>::from_be_bytes(slice.try_into()?);

            Ok(b)
        }
    };
}
