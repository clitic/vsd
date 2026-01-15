/*
    REFERENCES
    ----------

    1.
   https://github.com/axiomatic-systems/Bento4/blob/991d890908fb5da73704920c6de43480fb29f76a/Source/C%2B%2B/Apps/Mp4Decrypt/Mp4Decrypt.cpp

*/

#include "mp4decrypt.h"
#include "Ap4.h" // IWYU pragma: keep

int ap4_mp4decrypt(const unsigned char data[], unsigned int data_size,
                   const unsigned char *keys, unsigned int keys_count,
                   void *decrypted_data, callback_rust callback) {
  AP4_ProtectionKeyMap key_map;

  for (unsigned int i = 0; i < keys_count; i++) {
    const unsigned char *kid = keys + (i * 32);
    const unsigned char *key = keys + (i * 32) + 16;
    key_map.SetKeyForKid(kid, key, 16);
  }

  AP4_ByteStream *input = new AP4_MemoryByteStream(data, data_size);
  AP4_Processor *processor = new AP4_CencDecryptingProcessor(&key_map);
  AP4_MemoryByteStream *output = new AP4_MemoryByteStream();
  AP4_Result result = processor->Process(*input, *output, NULL);

  if (AP4_FAILED(result)) {
    return result;
  }

  delete processor;
  input->Release();
  callback(decrypted_data, output->GetData(), output->GetDataSize());
  output->Release();

  return 0;
}
