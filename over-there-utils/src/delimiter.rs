use std::future::Future;
use std::io;

pub const DEFAULT_DELIMITER: &[u8] = b"</>";

#[derive(Clone, Copy)]
struct PreReadResult {
    buf_len: usize,
    delimiter_len: usize,
}

/// Reader that takes closure to perform actual read, supporting sync and async
/// reads, buffering, and extracting data separated by a delimiter
pub struct DelimiterReader {
    /// Holds onto data that overflows past a delimiter
    buf: Box<[u8]>,

    /// Indicator of start of search for delimiter within buf
    buf_pos: usize,

    /// Indicator of how much of buffer is occupied with data
    buf_filled: usize,

    /// The delimiter to look for in read data
    pub delimiter: Vec<u8>,
}

impl DelimiterReader {
    pub fn new_with_delimiter(max_data_size: usize, delimiter: &[u8]) -> Self {
        Self {
            buf: vec![0; max_data_size + delimiter.len()].into_boxed_slice(),
            buf_pos: 0,
            buf_filled: 0,
            delimiter: delimiter.to_vec(),
        }
    }

    pub fn new(max_data_size: usize) -> Self {
        Self::new_with_delimiter(max_data_size, DEFAULT_DELIMITER)
    }

    /// Looks for a delimiter in the internal buffer that is not complete by
    /// searching backwards to find the largest partial delimiter; will
    /// never return more than a full delimiter and no less than 0
    ///
    /// (start position, size)
    fn find_partial_delimiter(&self) -> (usize, usize) {
        let b_len = self.buf.len();
        let delimiter_len = self.delimiter.len();
        let mut size = 0;
        let mut pos = 0;

        for i in (b_len - delimiter_len)..b_len {
            let l = b_len - i;
            if self.buf[i..] == self.delimiter[..l] {
                size = l;
                pos = i;
                break;
            }
        }

        (pos, size)
    }

    /// Performs movement of internal buffer data based on maximum capacity
    /// of buffer being reached
    fn pre_read(&mut self) -> PreReadResult {
        let buf_len = self.buf.len();
        let delimiter_len = self.delimiter.len();

        // If for some reason the buffer is completely full and we haven't
        // found our delimiter, we will shift by up to (not including) the
        // delimiter size and try again; this creates a sliding window where
        // we keep the max data size specified at all times
        if self.buf_pos > (buf_len - delimiter_len) {
            let (pd_pos, pdelimiter_len) = self.find_partial_delimiter();
            let shift_len = delimiter_len - pdelimiter_len;
            self.buf.rotate_left(shift_len);
            self.buf_filled -= shift_len;
            self.buf_pos = pd_pos - shift_len;
        };

        PreReadResult {
            buf_len,
            delimiter_len,
        }
    }

    /// Performs update to buffer based on read bytes, copying data to external
    /// buffer if we have found a delimiter and making room for new data
    fn post_read(
        &mut self,
        data: &mut [u8],
        bytes_read: Option<&usize>,
        preread_result: PreReadResult,
    ) -> usize {
        let PreReadResult {
            buf_len,
            delimiter_len,
        } = preread_result;

        // Mark where we will start reading and then update the filled count
        if let Some(bytes_read) = bytes_read {
            self.buf_filled += bytes_read;
        }

        // Scan for the delimiter starting from the last place searched
        let mut size = 0;
        if self.buf_filled > 0 {
            for i in self.buf_pos..=(buf_len - delimiter_len) {
                // If we have a match, we want to copy the contents (minus the delimiter) to the
                // provided buffer, shift over any remaining data, and reset our buf filled count
                if self.buf[i..i + delimiter_len] == self.delimiter[..] {
                    data[..i].copy_from_slice(&self.buf[..i]);
                    for j in &mut self.buf[..i + delimiter_len] {
                        *j = 0;
                    }
                    self.buf.rotate_left(i + delimiter_len);
                    self.buf_filled -= i + delimiter_len;
                    self.buf_pos = 0;
                    size = i;
                    break;
                }

                // Move buffer position after what we just checked
                self.buf_pos = i + 1;
            }
        }

        size
    }

    /// Performs an synchronous read using the given synchronous closure
    pub fn read<F>(&mut self, data: &mut [u8], f: F) -> io::Result<usize>
    where
        F: FnOnce(&mut [u8]) -> io::Result<usize>,
    {
        let preread_result = self.pre_read();

        // Attempt to fill up as much of buffer as possible without spilling over
        //
        // NOTE: This causes problems because we could have bytes still remaining
        //       in our buffer, but we will never get them because we exit
        //       immediately due to being unavailable; so, we wait to fully
        //       evaluate and return the error until the end in case we can
        //       process our buffer from existing data instead
        let read_result = f(&mut self.buf[self.buf_filled..]);

        let size =
            self.post_read(data, read_result.as_ref().ok(), preread_result);

        // If we didn't find anything new in our internal buffer and the read
        // result failed, we want to return the failure
        if size == 0 && read_result.is_err() {
            read_result
        } else {
            Ok(size)
        }
    }

    /// Performs an asynchronous read using the given asynchronous closure
    pub async fn async_read<R, F>(
        &mut self,
        data: &mut [u8],
        r: R,
    ) -> io::Result<usize>
    where
        R: FnOnce(&mut [u8]) -> F,
        F: Future<Output = io::Result<usize>>,
    {
        let preread_result = self.pre_read();

        // Attempt to fill up as much of buffer as possible without spilling over
        //
        // NOTE: This causes problems because we could have bytes still remaining
        //       in our buffer, but we will never get them because we exit
        //       immediately due to being unavailable; so, we wait to fully
        //       evaluate and return the error until the end in case we can
        //       process our buffer from existing data instead
        let read_result = r(&mut self.buf[self.buf_filled..]).await;

        let size =
            self.post_read(data, read_result.as_ref().ok(), preread_result);

        // If we didn't find anything new in our internal buffer and the read
        // result failed, we want to return the failure
        if size == 0 && read_result.is_err() {
            read_result
        } else {
            Ok(size)
        }
    }
}

/// Writer that takes closure to perform actual write, supporting sync and async
/// writes, tacking on a delimiter with each full write
pub struct DelimiterWriter {
    pub delimiter: Vec<u8>,
}

impl DelimiterWriter {
    pub fn new_with_delimiter(delimiter: &[u8]) -> Self {
        Self {
            delimiter: delimiter.to_vec(),
        }
    }

    pub fn new() -> Self {
        Self::new_with_delimiter(DEFAULT_DELIMITER)
    }

    pub fn write<F>(&mut self, data: &[u8], mut f: F) -> io::Result<usize>
    where
        F: FnMut(&[u8]) -> io::Result<usize>,
    {
        // Send all of the requested data first
        f(data)?;

        // Then send our delimiter
        f(&self.delimiter)?;

        Ok(data.len())
    }
}

impl Default for DelimiterWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn empty_nonblocking_read() -> impl FnOnce(&mut [u8]) -> io::Result<usize> {
        |_| Err(io::Error::from(io::ErrorKind::WouldBlock))
    }

    fn read_from_cursor<'a>(
        c: &'a mut Cursor<Vec<u8>>,
    ) -> impl FnOnce(&mut [u8]) -> io::Result<usize> + 'a {
        use std::io::Read;
        move |data| c.read(data)
    }

    fn write_from_vec<'a>(
        v: &'a mut Vec<u8>,
    ) -> impl FnMut(&[u8]) -> io::Result<usize> + 'a {
        use std::io::Write;
        move |data| v.write(data)
    }

    #[test]
    fn delimiter_reader_find_partial_delimiter_if_it_exists() {
        // Make a delimiter of 3 bytes and a data size of 3 byte, meaning
        // that internally the reader will grab 6 bytes at most at a time
        let delimiter = b"-+!";
        let max_data_size = 3;

        // First check that we properly return 0 for size if the buffer does
        // not contain the delimiter at all
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"000000");
        assert_eq!(r.find_partial_delimiter(), (0, 0));

        // Second check that we properly return 0 for size if the buffer does
        // not partially end with the delimiter; we don't check the beginning
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"-+!000");
        assert_eq!(r.find_partial_delimiter(), (0, 0));

        // Third check that we properly return 0 for size if the buffer does
        // not partially end with the delimiter; we don't get tripped up by
        // part of a delimiter mid-way
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"000-+0");
        assert_eq!(r.find_partial_delimiter(), (0, 0));

        // Fourth, check that we properly match a single byte of the delimiter
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"00000-");
        assert_eq!(
            r.find_partial_delimiter(),
            (5, 1),
            "Failed to find first byte of delimiter"
        );

        // Fifth, check that we properly match multiple bytes of the delimiter
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"0000-+");
        assert_eq!(
            r.find_partial_delimiter(),
            (4, 2),
            "Failed to find multiple bytes of delimiter"
        );

        // Sixth, check that we properly match the delimiter at the end
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"000-+!");
        assert_eq!(
            r.find_partial_delimiter(),
            (3, 3),
            "Failed to find entire delimiter"
        );

        // Sixth, check that we properly match the final delimiter, not one earlier
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"-+!-+!");
        assert_eq!(
            r.find_partial_delimiter(),
            (3, 3),
            "Failed to find last entire delimiter"
        );

        // Seventh, check that we properly match the final partial delimiter, not one earlier
        let mut r =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        r.buf.copy_from_slice(b"0-+!-+");
        assert_eq!(
            r.find_partial_delimiter(),
            (4, 2),
            "Failed to find last partial delimiter"
        );
    }

    #[test]
    fn delimiter_reader_should_fill_provided_buffer_if_found_delimiter() {
        let delimiter = b"</test>";
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut cursor = {
            let mut reader = Vec::new();
            reader.extend_from_slice(&data);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(data.len(), delimiter);

        // Perform the read, gathering all of the data at once
        let mut buf = vec![0; data.len()];
        assert_eq!(
            delimiter_reader
                .read(&mut buf, read_from_cursor(&mut cursor))
                .unwrap(),
            data.len()
        );
        assert_eq!(buf, data);
    }

    #[test]
    fn delimiter_reader_should_support_data_less_than_max_size() {
        let delimiter = b"</test>";
        let max_data_size = 10;
        let mut cursor = {
            let mut reader = Vec::new();
            reader.extend(vec![1]);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);

        // Read first data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![1][..]);
        assert_eq!(buf[size..], vec![0; max_data_size - size][..]);
    }

    #[test]
    fn delimiter_reader_should_support_data_more_than_max_size_by_truncating_earlier_data(
    ) {
        let delimiter = b"</test>";
        let max_data_size = 3;
        let mut cursor = {
            let mut reader = Vec::new();
            reader.extend(vec![1, 2, 3, 4, 5]);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        let mut buf = vec![0; max_data_size];

        // First read cannot fit all data and thereby doesn't find the delimiter,
        // so will yield 0 bytes
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(size, 0);

        // Second read should acquire the remainder of the data, find the delimiter,
        // and yield the read data size
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![3, 4, 5][..]);
    }

    #[test]
    fn delimiter_reader_should_support_multiple_delimiters_being_encountered() {
        let delimiter = b"</test>";
        let max_data_size = 3;
        let mut cursor = {
            let mut reader = Vec::new();
            reader.extend(vec![1]);
            reader.extend_from_slice(delimiter);
            reader.extend(vec![4, 5, 6]);
            reader.extend_from_slice(delimiter);
            reader.extend(vec![2, 3]);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);

        // Read first data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![1][..]);

        // Read second data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![4, 5, 6][..]);

        // Read third data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![2, 3][..]);
    }

    #[test]
    fn delimiter_reader_should_support_multiple_delimited_chunks_in_one_read() {
        let delimiter = b"</test>";

        // Make the max size (of our internal buffer) much bigger than all data
        let max_data_size = 100;
        let mut cursor = {
            let mut reader = Vec::new();
            reader.extend(vec![1]);
            reader.extend_from_slice(delimiter);
            reader.extend(vec![4, 5, 6]);
            reader.extend_from_slice(delimiter);
            reader.extend(vec![2, 3]);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);

        // Read first data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![1][..]);

        // Read second data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![4, 5, 6][..]);

        // Read third data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader
            .read(&mut buf, read_from_cursor(&mut cursor))
            .unwrap();
        assert_eq!(buf[..size], vec![2, 3][..]);
    }

    #[test]
    fn delimiter_reader_should_continue_using_internal_buffer_even_if_internal_reader_fails(
    ) {
        // Delimiter (7 bytes) * 2 + Data (1 byte) * 2 = 2 writes
        let delimiter = b"</test>";
        let max_data_size = 9;

        // Prep our reader to have a certain state where data is still available
        // internally but the underlying reader will always yield an error
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(max_data_size, delimiter);
        delimiter_reader.buf.copy_from_slice(b"0</test>1</test>");
        delimiter_reader.buf_pos = 0;
        delimiter_reader.buf_filled = delimiter_reader.buf.len();

        let mut buf = vec![0; max_data_size];

        let size = delimiter_reader
            .read(&mut buf, empty_nonblocking_read())
            .unwrap();
        assert_eq!(&buf[..size], b"0");

        let size = delimiter_reader
            .read(&mut buf, empty_nonblocking_read())
            .unwrap();
        assert_eq!(&buf[..size], b"1");

        let result = delimiter_reader.read(&mut buf, empty_nonblocking_read());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::WouldBlock);
    }

    #[test]
    fn delimiter_writer_should_send_all_bytes_and_append_the_delimiter() {
        let delimiter = b"</test>";
        let mut writer: Vec<u8> = Vec::new();
        let mut delimiter_writer =
            DelimiterWriter::new_with_delimiter(delimiter);
        let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        // Size should be the data sent not including the delimiter
        assert_eq!(
            delimiter_writer
                .write(&data, write_from_vec(&mut writer))
                .unwrap(),
            data.len()
        );

        // Result should be the data and a delimiter appended
        data.extend_from_slice(delimiter);
        assert_eq!(&writer, &data);
    }
}
