/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

/// There are two implementations of this trait:
/// - [super::RingBufferStack] which uses a fixed-size array on the stack.
/// - [super::RingBufferHeap] which uses a [Vec] on the heap.
pub trait RingBuffer<T, const N: usize> {
    fn len(&self) -> usize;
    fn clear(&mut self);
    fn get(&self, index: usize) -> Option<&T>;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn first(&self) -> Option<&T> { self.get(0) }
    fn last(&self) -> Option<&T> { self.get(self.len().saturating_sub(1)) }
    fn add(&mut self, value: T);
    fn remove(&mut self) -> Option<T>;
    fn remove_head(&mut self) -> Option<T>;
    fn truncate(&mut self, index: usize);
}
