pub trait Integer
where
    Self: Sized,
{
    fn size() -> usize;
    fn from_bytes(bytes: &[u8]) -> Option<(Self, &[u8])>;
}

macro_rules! number_impl {
    ($t:ty) => {
        impl Integer for $t {
            fn size() -> usize {
                ::core::mem::size_of::<$t>()
            }

            fn from_bytes(bytes: &[u8]) -> Option<(Self, &[u8])> {
                let (int_bytes, rest) = bytes.split_at_checked(Self::size())?;

                int_bytes
                    .try_into()
                    .ok()
                    .map(|b| (<$t>::from_le_bytes(b), rest))
            }
        }
    };
}

number_impl!(u8);
number_impl!(u16);
number_impl!(u32);
number_impl!(u64);
number_impl!(u128);
number_impl!(usize);

number_impl!(i8);
number_impl!(i16);
number_impl!(i32);
number_impl!(i64);
number_impl!(i128);
number_impl!(isize);

pub struct BytesIter<'a> {
    pub bytes: &'a [u8],
}

impl BytesIter<'_> {
    pub fn read<T: Integer>(&mut self) -> Option<T> {
        let (value, rest) = T::from_bytes(self.bytes)?;
        self.bytes = rest;
        Some(value)
    }
}

impl Iterator for BytesIter<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.read()
    }
}
