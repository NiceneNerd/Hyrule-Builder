use crate::{AnyError, GeneralResult};
use std::io::Write;

pub fn compress<B: AsRef<[u8]>>(source: B) -> GeneralResult<Vec<u8>> {
    let source = source.as_ref();
    //Yaz0 check
    if source.len() < 0x0F {
        return Err(Box::<AnyError>::from("Insufficient data"));
    }
    if &source[0..4] == b"Yaz0" {
        return Err(Box::<AnyError>::from("Already compressed".to_owned()));
    }
    let uncompressed_size: usize = source.len(); //0x04
    let data_offset: u32 =
        u32::from_le_bytes([source[0x0C], source[0x0D], source[0x0E], source[0x0F]]);

    //Encode Logic
    let mut source_pos: usize = 0; //start of data after header
    let mut write_pos: usize = 0;
    let mut group_pos: usize;
    let mut copy_pos: usize;
    let mut encoded_data: Vec<u8> =
        vec![0; uncompressed_size + (uncompressed_size as f32 / 2.) as usize];
    let mut group_data: Vec<u8>;
    let mut group_header: u8;
    let mut group_header_flag: String;
    let mut num_bytes: usize = 0;
    let mut rle_num_bytes: usize;
    let mut copy_num_bytes: usize;
    let mut predict_num_bytes: usize;
    let mut predict_copy_pos: usize;
    let mut predict_hit: bool = false;
    let mut buffer_pos: usize;
    let mut seek_pos: usize;
    let mut data_calc: u16;
    let fixed_offset: usize;
    if data_offset == 0x2000 {
        fixed_offset = data_offset as usize;
    } else {
        fixed_offset = 0x00;
    }
    //for (int i = 0; i < 190; i += 1) //debug
    while source_pos < uncompressed_size {
        group_data = vec![0; 24];
        group_header_flag = "".to_owned();
        group_pos = 0; //first byte
        while group_header_flag.len() < 8
        //ensure number of Header Flags is less than 8, as group can be between 8-16 bytes
        {
            rle_num_bytes = 3;
            copy_num_bytes = 3;
            predict_num_bytes = 3; // reset
            copy_pos = 0;
            predict_copy_pos = 0;
            let mut is_match: bool = false;

            if source_pos != 0 && source_pos + 3 < uncompressed_size {
                //RLE check
                if (source[source_pos] == source[source_pos - 1])
                    && (source[source_pos + 1] == source[source_pos - 1])
                    && (source[source_pos + 2] == source[source_pos - 1])
                    && !predict_hit
                //is_match found for RLE/overlap
                {
                    is_match = true;
                    buffer_pos = source_pos + 3; //buffer source ahead
                    copy_pos = source_pos - 1;
                    if buffer_pos < uncompressed_size {
                        while buffer_pos < uncompressed_size
                            && source[buffer_pos] == source[source_pos - 1]
                            && rle_num_bytes < (0xFF + 0xF + 3)
                        //while there is more data matching from that one byte... (don't ask about the math plz, even I am confused)
                        {
                            rle_num_bytes += 1;
                            buffer_pos += 1;
                        }
                    }
                }
                //Copy check
                let mut back_pos = source_pos - 1;
                while back_pos > 0 && (source_pos - back_pos) < 0xFFF
                //go backwards into the source data from current position and search for a matching pattern
                {
                    if source[source_pos] == source[back_pos]
                        && source[source_pos + 1] == source[back_pos + 1]
                        && source[source_pos + 2] == source[back_pos + 2]
                    //is_match found for copy
                    {
                        is_match = true;
                        seek_pos = back_pos + 3; //search ahead
                        buffer_pos = source_pos + 3; //buffer source ahead

                        if copy_pos == 0 {
                            //if there is no copy position recorded...
                            copy_pos = back_pos;
                        }

                        let mut instance_num_bytes: usize = 4;
                        if buffer_pos < uncompressed_size && seek_pos < uncompressed_size {
                            while buffer_pos < uncompressed_size
                                && seek_pos < uncompressed_size
                                && source[buffer_pos] == source[seek_pos]
                                && copy_num_bytes < (0xFF + 0xF + 3)
                            //while there is more data matched, and the seek position is less than the source position...
                            {
                                if copy_pos != back_pos
                                //if new potential position is found
                                {
                                    if copy_num_bytes < instance_num_bytes
                                    //if current num_bytes is less than new instance, take new position and increment
                                    {
                                        copy_pos = back_pos;
                                        copy_num_bytes += 1;
                                    }
                                } else {
                                    copy_num_bytes += 1;
                                }
                                instance_num_bytes += 1;
                                seek_pos += 1;
                                buffer_pos += 1;
                            }
                        }
                    }
                    if source[source_pos + 1] == source[back_pos]
                        && source[source_pos + 2] == source[back_pos + 1]
                        && source[source_pos + 3] == source[back_pos + 2]
                    //Predict
                    {
                        seek_pos = back_pos + 3; //search ahead
                        buffer_pos = source_pos + 4; //buffer source ahead, predicted

                        if predict_copy_pos == 0 {
                            //if there is no copy position recorded...
                            predict_copy_pos = back_pos;
                        }

                        let mut instance_num_bytes: usize = 4;
                        if buffer_pos < uncompressed_size && seek_pos < uncompressed_size {
                            while buffer_pos < uncompressed_size
                                && seek_pos < uncompressed_size
                                && source[buffer_pos] == source[seek_pos]
                                && predict_num_bytes < (0xFF + 0xF + 3)
                            {
                                if predict_copy_pos != back_pos
                                //if new potential position is found
                                {
                                    if predict_num_bytes < instance_num_bytes
                                    //if current num_bytes is less than new instance, take new position and increment num_bytes
                                    {
                                        predict_copy_pos = back_pos;
                                        predict_num_bytes += 1;
                                    }
                                } else {
                                    predict_num_bytes += 1;
                                }
                                instance_num_bytes += 1;
                                seek_pos += 1;
                                buffer_pos += 1;
                            }
                        }
                    }

                    //if (source_pos >= 0x3DA9 && (source_pos - back_pos) > 3835) //debug encode
                    //System.Windows.Forms.MessageBox.Show("source_pos: 0x" + source_pos.ToString("X") + "\n" + "searchPos: " + "0x" + back_pos.ToString("X") + "\n" + "copy_pos: 0x" + copy_pos.ToString("X") + "\n" + "predict_copy_pos: 0x" + predict_copy_pos.ToString("X") + "\n" + "dist: " + (source_pos - back_pos) + "\n" + "copy_num_bytes: " + copy_num_bytes + "\n" + "predict_num_bytes: " + predict_num_bytes);
                    back_pos -= 1;
                }
                predict_hit = false; //reset prediction
                if rle_num_bytes >= copy_num_bytes {
                    //use RLE number of bytes unless copy_num_bytes found a better is_match
                    num_bytes = rle_num_bytes;
                } else {
                    num_bytes = copy_num_bytes;
                }
                if predict_num_bytes > num_bytes {
                    is_match = false; //flag the next byte as straight copy because the next one will solve one copy instead of two. (End up using 3 bytes instead of 4)
                    predict_hit = true;
                }
            }
            if is_match
            //Flag for RLE/copy
            {
                if num_bytes > 18 {
                    data_calc = (((0x0) << 12) | (source_pos - copy_pos) - 1) as u16;
                }
                //Mark the 4-bits all 0 to reference the 3rd byte to copy
                else {
                    data_calc = (((num_bytes - 2) << 12) | (source_pos - copy_pos) - 1) as u16;
                } //Calculate the pair
                group_data[group_pos] = (data_calc >> 8) as u8; //b1
                group_data[group_pos + 1] = (data_calc & 0xFF) as u8; //b2
                group_pos += 2;
                source_pos += num_bytes; //add by how many copies
                group_header_flag.push('0');
                if num_bytes >= 18
                //if num_bytes is greater than 18, but it will be used to accomodate the large number of bytes to copy, do not flag nor increment as it's part of the pair
                {
                    group_data[group_pos] = (num_bytes - 18) as u8;
                    group_pos += 1;
                }
            } else if source_pos + 1 > uncompressed_size
            //End of encryption
            {
                group_header_flag.push('0');
                source_pos += 1;
            } else {
                //Flag for Straight copy
                group_data[group_pos] = source[source_pos];
                group_pos += 1;
                source_pos += 1;
                group_header_flag.push('1');
            }
        } //end while

        group_header = u8::from_str_radix(&group_header_flag, 2).unwrap();

        if write_pos < encoded_data.len() {
            encoded_data[write_pos] = group_header;
        } else {
            encoded_data.insert(write_pos, group_header);
        }
        write_pos += 1;
        for v in group_data {
            if write_pos < encoded_data.len() {
                encoded_data[write_pos] = v;
            } else {
                encoded_data.insert(write_pos, v);
            }
            write_pos += 1;
        }
    } //end while
      //Write all of the encoded data
    let mut data: Vec<u8> = vec![];
    data.write_all(b"Yaz0")?;
    data.write_all(&uncompressed_size.to_be_bytes())?;
    data.write_all(&fixed_offset.to_le_bytes())?;
    data.write_all(&[0, 0, 0, 0])?;
    data.write_all(&encoded_data)?;
    Ok(data)
}
