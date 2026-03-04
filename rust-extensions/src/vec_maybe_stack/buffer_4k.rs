use super::StackBuffer;

const BUFFER_LEN: usize = 4096;
pub struct Buffer4K<T: Default + Copy + Clone> {
    buffer: [T; BUFFER_LEN],
    len: usize,
}

impl<T: Default + Copy + Clone> Default for Buffer4K<T> {
    fn default() -> Self {
        Self {
            buffer: [T::default(); BUFFER_LEN],
            len: 0,
        }
    }
}

impl<T: Default + Copy + Clone> StackBuffer<T> for Buffer4K<T> {
    const STACK_CAPACITY: usize = BUFFER_LEN;

    fn len(&self) -> usize {
        self.len
    }

    fn get_mut_slice(&mut self) -> &mut [T] {
        self.buffer.as_mut_slice()
    }

    fn get_slice(&self) -> &[T] {
        &self.buffer[..self.len]
    }

    fn increment_len(&mut self, amount: usize) {
        self.len += amount;
    }
}
