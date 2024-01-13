use super::VecMaybeStackIterator;

pub struct VecMaybeStack<T, TStackBuffer: StackBuffer<T> + Default> {
    stack: TStackBuffer,
    heap: Vec<T>,
}

impl<T: Clone + Copy, TStackBuffer: StackBuffer<T> + Default> VecMaybeStack<T, TStackBuffer> {
    pub fn new() -> Self {
        Self {
            stack: TStackBuffer::default(),
            heap: Vec::new(),
        }
    }

    pub fn push(&mut self, value: T) {
        let stack_len = self.stack.len();

        if stack_len < TStackBuffer::STACK_CAPACITY {
            self.stack.get_mut_full_slice()[stack_len] = value;
            self.stack.increment_len(1);
            return;
        }

        self.heap.push(value);
    }

    pub fn push_slice(&mut self, slice: &[T]) {
        let stack_len = self.stack.len();

        if stack_len == TStackBuffer::STACK_CAPACITY {
            self.heap.extend_from_slice(slice);
            return;
        }

        if stack_len + slice.len() <= TStackBuffer::STACK_CAPACITY {
            self.stack.get_mut_full_slice()[stack_len..stack_len + slice.len()]
                .copy_from_slice(slice);
            self.stack.increment_len(slice.len());
            return;
        }

        if stack_len == TStackBuffer::STACK_CAPACITY {
            self.heap.extend_from_slice(slice);
            return;
        }

        let goes_to_stack = TStackBuffer::STACK_CAPACITY - stack_len;

        self.stack.get_mut_full_slice()[stack_len..].copy_from_slice(&slice[..goes_to_stack]);
        self.stack.increment_len(goes_to_stack);

        self.heap.extend_from_slice(&slice[goes_to_stack..]);
    }

    pub fn len(&self) -> usize {
        return self.heap.len() + self.stack.len();
    }
    pub fn to_vec(&self) -> Vec<T> {
        let mut result = self.stack.get_slice().to_vec();

        if self.heap.len() > 0 {
            result.extend_from_slice(self.heap.as_slice());
        }

        result
    }

    /*
    pub fn slice<'s>(&'s self) -> MaybeStackSlice<'s, T> {
        if self.heap.len() > 0 {
            return MaybeStackSlice::new(Some(self.stack.get_slice()), Some(self.heap.as_slice()));
        }

        return MaybeStackSlice::new(Some(&self.stack.get_slice()[..self.stack.len()]), None);
    }

     */

    pub fn iter<'a>(&'a self) -> VecMaybeStackIterator<'a, T> {
        return VecMaybeStackIterator::new(
            self.stack.get_slice().iter(),
            self.heap.as_slice().iter(),
        );
    }
}

pub trait StackBuffer<T> {
    const STACK_CAPACITY: usize;

    fn len(&self) -> usize;
    fn get_mut_full_slice(&mut self) -> &mut [T];
    fn get_slice(&self) -> &[T];

    fn increment_len(&mut self, amount: usize);
}

#[cfg(test)]
mod tests {
    use crate::vec_maybe_stack::Buffer32;

    use super::VecMaybeStack;

    #[test]
    fn test_maybe_stack_32() {
        let mut src_vec: VecMaybeStack<u8, Buffer32<u8>> = VecMaybeStack::new();

        let mut src_as_usual_vec = Vec::new();

        for i in 0..64 {
            src_vec.push(i);
            src_as_usual_vec.push(i);
        }

        let mut dest = Vec::new();

        for itm in src_vec.iter() {
            dest.push(*itm)
        }
    }
}
