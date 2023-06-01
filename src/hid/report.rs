// SPDX-FileCopyrightText: The djio authors
// SPDX-License-Identifier: MPL-2.0

#[derive(Debug)]
pub struct BufferRecycler {
    // Recycle report buffers, one slot per report id
    recycled: Vec<Vec<Vec<u8>>>,
}

impl BufferRecycler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            // One slot per report id
            recycled: std::iter::repeat(Vec::new())
                .take(usize::from(u8::MAX) + 1)
                .collect(),
        }
    }

    #[must_use]
    pub fn try_fetch_buf(&mut self, report_id: u8) -> Option<Vec<u8>> {
        let index = usize::from(report_id);
        debug_assert!(index < self.recycled.len());
        #[allow(unsafe_code)]
        unsafe { self.recycled.get_unchecked_mut(index) }.pop()
    }

    #[must_use]
    pub fn fill_buf(&mut self, data: &[u8]) -> Vec<u8> {
        let report_id = data[0];
        if let Some(mut recycled) = self.try_fetch_buf(report_id) {
            debug_assert_eq!(recycled[0], report_id);
            // All reports of the same id usually have the same length and
            // resizing won't have any affect. This is also the reason why
            // we have picked an arbitrary buffer from those that have been
            // recycled.
            let old_len = recycled.len();
            let new_len = data.len();
            if old_len != new_len {
                log::debug!("Resizing recycled buffer from {old_len} to {new_len}");
            }
            if new_len <= old_len {
                recycled.truncate(new_len);
                recycled.copy_from_slice(data);
            } else {
                recycled.copy_from_slice(&data[..old_len]);
                recycled.extend_from_slice(&data[old_len..]);
            }
            recycled
        } else {
            data.to_vec()
        }
    }

    pub fn recycle_buf(&mut self, buffer: Vec<u8>) {
        let report_id = buffer[0];
        let index = usize::from(report_id);
        debug_assert!(index < self.recycled.len());
        #[allow(unsafe_code)]
        unsafe { self.recycled.get_unchecked_mut(index) }.push(buffer);
    }
}

impl Default for BufferRecycler {
    fn default() -> Self {
        Self::new()
    }
}
