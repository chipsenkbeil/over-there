use std::io::{self, BufReader, BufWriter, Read, Write};

pub const DEFAULT_DELIMITER: &[u8] = b"</>";

pub struct DelimiterReader<T>
where
    T: Read,
{
    buf_reader: BufReader<T>,

    /// Holds onto data that overflows past a delimiter
    pub buf: Box<[u8]>,

    /// Indicator of start of search for delimiter within buf
    buf_pos: usize,

    /// Indicator of how much of buffer is occupied with data
    buf_filled: usize,

    /// The delimiter to look for in read data
    delimiter: Vec<u8>,
}

impl<T> DelimiterReader<T>
where
    T: Read,
{
    pub fn new_with_delimiter(read: T, max_data_size: usize, delimiter: &[u8]) -> Self {
        let buf_reader = BufReader::new(read);

        Self {
            buf_reader,
            buf: vec![0; max_data_size + delimiter.len()].into_boxed_slice(),
            buf_pos: 0,
            buf_filled: 0,
            delimiter: delimiter.to_vec(),
        }
    }

    pub fn new(read: T, max_data_size: usize) -> Self {
        Self::new_with_delimiter(read, max_data_size, DEFAULT_DELIMITER)
    }

    /// Looks for a delimiter in the internal buffer that is not complete by
    /// searching backwards to find the largest partial delimiter; will
    /// never return more than a full delimiter and no less than 0
    ///
    /// (start position, size)
    fn find_partial_delimiter(&self) -> (usize, usize) {
        let b_len = self.buf.len();
        let d_len = self.delimiter.len();
        let mut size = 0;
        let mut pos = 0;

        for i in (b_len - d_len)..b_len {
            let l = b_len - i;
            if self.buf[i..] == self.delimiter[..l] {
                size = l;
                pos = i;
                break;
            }
        }

        (pos, size)
    }
}

impl<T> Read for DelimiterReader<T>
where
    T: Read,
{
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        let buf_len = self.buf.len();
        let d_len = self.delimiter.len();

        // If for some reason the buffer is completely full and we haven't
        // found our delimiter, we will shift by up to (not including) the
        // delimiter size and try again; this creates a sliding window where
        // we keep the max data size specified at all times
        if buf_len == self.buf_filled {
            let (pd_pos, pd_len) = self.find_partial_delimiter();
            let shift_len = d_len - pd_len;
            self.buf.rotate_left(shift_len);
            self.buf_filled -= shift_len;
            self.buf_pos = pd_pos - shift_len;
        };

        // Attempt to fill up as much of buffer as possible without spilling over
        let bytes_read = self.buf_reader.read(&mut self.buf[self.buf_filled..])?;

        // Mark where we will start reading and then update the filled count
        self.buf_filled += bytes_read;

        // Scan for the delimiter starting from the last place searched
        let mut size = 0;
        for i in self.buf_pos..=(buf_len - d_len) {
            self.buf_pos = i;

            // If we have a match, we want to copy the contents (minus the delimiter) to the
            // provided buffer, shift over any remaining data, and reset our buf filled count
            if self.buf[i..i + d_len] == self.delimiter[..] {
                data[0..i].copy_from_slice(&self.buf[0..i]);
                self.buf.rotate_left(i + d_len);
                self.buf_filled -= i + d_len;
                self.buf_pos = 0;
                size = i;
                break;
            }
        }

        Ok(size)
    }
}

pub struct DelimiterWriter<T>
where
    T: Write,
{
    buf_writer: BufWriter<T>,
    delimiter: Vec<u8>,
}

impl<T> DelimiterWriter<T>
where
    T: Write,
{
    pub fn new_with_delimiter(write: T, delimiter: &[u8]) -> Self {
        let buf_writer = BufWriter::new(write);

        Self {
            buf_writer,
            delimiter: delimiter.to_vec(),
        }
    }

    pub fn new(write: T) -> Self {
        Self::new_with_delimiter(write, DEFAULT_DELIMITER)
    }
}

impl<T> Write for DelimiterWriter<T>
where
    T: Write,
{
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        // Send all of the requested data first
        self.buf_writer.write_all(data)?;

        // Then send our delimiter
        self.buf_writer.write_all(&self.delimiter)?;

        // Then enforce sending all queued data
        self.buf_writer.flush()?;

        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf_writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn delimiter_reader_find_partial_delimiter_if_it_exists() {
        // Make a delimiter of 3 bytes and a data size of 3 byte, meaning
        // that internally the reader will grab 6 bytes at most at a time
        let delimiter = b"-+!";
        let max_data_size = 3;

        // First check that we properly return 0 for size if the buffer does
        // not contain the delimiter at all
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
        r.buf.copy_from_slice(b"000000");
        assert_eq!(r.find_partial_delimiter(), (0, 0));

        // Second check that we properly return 0 for size if the buffer does
        // not partially end with the delimiter; we don't check the beginning
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
        r.buf.copy_from_slice(b"-+!000");
        assert_eq!(r.find_partial_delimiter(), (0, 0));

        // Third check that we properly return 0 for size if the buffer does
        // not partially end with the delimiter; we don't get tripped up by
        // part of a delimiter mid-way
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
        r.buf.copy_from_slice(b"000-+0");
        assert_eq!(r.find_partial_delimiter(), (0, 0));

        // Fourth, check that we properly match a single byte of the delimiter
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
        r.buf.copy_from_slice(b"00000-");
        assert_eq!(
            r.find_partial_delimiter(),
            (5, 1),
            "Failed to find first byte of delimiter"
        );

        // Fifth, check that we properly match multiple bytes of the delimiter
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
        r.buf.copy_from_slice(b"0000-+");
        assert_eq!(
            r.find_partial_delimiter(),
            (4, 2),
            "Failed to find multiple bytes of delimiter"
        );

        // Sixth, check that we properly match the delimiter at the end
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
        r.buf.copy_from_slice(b"000-+!");
        assert_eq!(
            r.find_partial_delimiter(),
            (3, 3),
            "Failed to find entire delimiter"
        );

        // Sixth, check that we properly match the final delimiter, not one earlier
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
        r.buf.copy_from_slice(b"-+!-+!");
        assert_eq!(
            r.find_partial_delimiter(),
            (3, 3),
            "Failed to find last entire delimiter"
        );

        // Seventh, check that we properly match the final partial delimiter, not one earlier
        let mut r = DelimiterReader::new_with_delimiter(Cursor::new(b""), max_data_size, delimiter);
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
        let reader: Cursor<Vec<u8>> = {
            let mut reader = Vec::new();
            reader.extend_from_slice(&data);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(reader, data.len(), delimiter);

        // Perform the read, gathering all of the data at once
        let mut buf = vec![0; data.len()];
        assert_eq!(delimiter_reader.read(&mut buf).unwrap(), data.len());
        assert_eq!(buf, data);
    }

    #[test]
    fn delimiter_reader_should_support_data_less_than_max_size() {
        let delimiter = b"</test>";
        let max_data_size = 10;
        let reader: Cursor<Vec<u8>> = {
            let mut reader = Vec::new();
            reader.extend(vec![1]);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(reader, max_data_size, delimiter);

        // Read first data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![1][..]);
        assert_eq!(buf[size..], vec![0; max_data_size - size][..]);
    }

    #[test]
    fn delimiter_reader_should_support_data_more_than_max_size_by_truncating_earlier_data() {
        let delimiter = b"</test>";
        let max_data_size = 3;
        let reader: Cursor<Vec<u8>> = {
            let mut reader = Vec::new();
            reader.extend(vec![1, 2, 3, 4, 5]);
            reader.extend_from_slice(delimiter);
            Cursor::new(reader)
        };

        // Create our reader that supports the entire size of our data,
        // not including the delimiter
        let mut delimiter_reader =
            DelimiterReader::new_with_delimiter(reader, max_data_size, delimiter);
        let mut buf = vec![0; max_data_size];

        // First read cannot fit all data and thereby doesn't find the delimiter,
        // so will yield 0 bytes
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(size, 0);

        // Second read should acquire the remainder of the data, find the delimiter,
        // and yield the read data size
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![3, 4, 5][..]);
    }

    #[test]
    fn delimiter_reader_should_support_multiple_delimiters_being_encountered() {
        let delimiter = b"</test>";
        let max_data_size = 3;
        let reader: Cursor<Vec<u8>> = {
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
            DelimiterReader::new_with_delimiter(reader, max_data_size, delimiter);

        // Read first data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![1][..]);

        // Read second data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![4, 5, 6][..]);

        // Read third data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![2, 3][..]);
    }

    #[test]
    fn delimiter_reader_should_support_multiple_delimited_chunks_in_one_read() {
        let delimiter = b"</test>";

        // Make the max size (of our internal buffer) much bigger than all data
        let max_data_size = 100;
        let reader: Cursor<Vec<u8>> = {
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
            DelimiterReader::new_with_delimiter(reader, max_data_size, delimiter);

        // Read first data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![1][..]);

        // Read second data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![4, 5, 6][..]);

        // Read third data with delimiter
        let mut buf = vec![0; max_data_size];
        let size = delimiter_reader.read(&mut buf).unwrap();
        assert_eq!(buf[..size], vec![2, 3][..]);
    }

    #[test]
    fn delimiter_writer_should_send_all_bytes_and_append_the_delimiter() {
        let delimiter = b"</test>";
        let writer: Vec<u8> = Vec::new();
        let mut delimiter_writer = DelimiterWriter::new_with_delimiter(writer, delimiter);
        let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        // Size should be the data sent not including the delimiter
        assert_eq!(delimiter_writer.write(&data).unwrap(), data.len());

        // Result should be the data and a delimiter appended
        data.extend_from_slice(delimiter);
        assert_eq!(delimiter_writer.buf_writer.get_ref(), &data);
    }
}
