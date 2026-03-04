pub struct MaybeStackSlice<'s, T: Copy + Clone> {
    stack: Option<&'s [T]>,
    heap: Option<&'s [T]>,
}

impl<'s, T: Copy + Clone> MaybeStackSlice<'s, T> {
    pub fn new(stack: Option<&'s [T]>, heap: Option<&'s [T]>) -> Self {
        Self { stack, heap }
    }

    pub fn get(&'s self, index: usize) -> Option<&'s T> {
        let mut offset = 0;

        if let Some(stack) = self.stack {
            if index < stack.len() {
                return Some(&stack[index]);
            }

            offset += stack.len();
        }

        if let Some(heap) = self.heap {
            let heap_offset = index - offset;
            if heap_offset < heap.len() {
                return Some(&heap[heap_offset]);
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        let mut result = 0;

        if let Some(stack) = self.stack {
            result += stack.len();
        }

        if let Some(heap) = self.heap {
            result += heap.len();
        }

        result
    }

    pub fn write_to_vec(&self, vec: &mut Vec<T>) {
        if let Some(stack) = self.stack {
            vec.extend_from_slice(stack);
        }

        if let Some(heap) = self.heap {
            vec.extend_from_slice(heap);
        }
    }

    pub fn iter(&'s self) -> MaybeStackSliceIterator<'s, T> {
        MaybeStackSliceIterator {
            stack: self.stack,
            heap: self.heap,
            stack_position: 0,
            heap_position: 0,
        }
    }
}

pub struct MaybeStackSliceIterator<'s, T> {
    stack: Option<&'s [T]>,
    heap: Option<&'s [T]>,
    stack_position: usize,
    heap_position: usize,
}

impl<'s, T> Iterator for MaybeStackSliceIterator<'s, T> {
    type Item = &'s T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(stack) = self.stack {
            if self.stack_position < stack.len() {
                let result = &stack[self.stack_position];
                self.stack_position += 1;
                return Some(result);
            }
        }

        if let Some(heap) = self.heap {
            if self.heap_position < heap.len() {
                let result = &heap[self.heap_position];
                self.heap_position += 1;
                return Some(result);
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use super::MaybeStackSliceIterator;

    #[test]
    fn test_iterator_basic_case() {
        let stack = [1u8, 2u8, 3u8, 4u8, 5u8];
        let heap = [6u8, 7u8, 8u8, 9u8, 10u8];
        let test_iterator = MaybeStackSliceIterator {
            stack: Some(&stack),
            heap: Some(&heap),
            stack_position: 0,
            heap_position: 0,
        };

        let mut result: Vec<u8> = Vec::new();

        result.extend(test_iterator);

        assert_eq!(
            result,
            vec![1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8]
        );
    }
}
