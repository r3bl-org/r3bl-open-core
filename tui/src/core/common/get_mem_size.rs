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

pub trait GetMemSize {
    fn get_mem_size(&self) -> usize;
}

/// Calculates the total memory size of a slice of items that implement [GetMemSize].
/// This is useful when you need to calculate the total memory size of a collection of
/// items (eg [Vec] or [smallvec::SmallVec] of [crate::GCString]).
pub fn slice_size<T: GetMemSize>(slice: &[T]) -> usize {
    slice.iter().map(|item| item.get_mem_size()).sum::<usize>()
}

/// Calculates the total memory size of an iterator of items that implement
/// [GetMemSize]. This is useful when you need to calculate the total memory size of
/// an iterator of items (eg: from [crate::RingBufferHeap] or
/// [crate::RingBufferStack]) that contains items that are [Option] of [GetMemSize].
pub fn iter_size<T: GetMemSize, I: Iterator<Item = Option<T>>>(iter: I) -> usize {
    iter.map(|item| item.as_ref().map_or(0, |item| item.get_mem_size()))
        .sum::<usize>()
}

pub fn ring_buffer_size<T: GetMemSize, const N: usize>(
    ring_buffer: &crate::RingBufferHeap<T, N>,
) -> usize {
    ring_buffer
        .iter()
        .map(|item| item.get_mem_size())
        .sum::<usize>()
}
