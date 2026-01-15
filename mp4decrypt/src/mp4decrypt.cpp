/*
    REFERENCES
    ----------

    1.
   https://github.com/axiomatic-systems/Bento4/blob/991d890908fb5da73704920c6de43480fb29f76a/Source/C%2B%2B/Apps/Mp4Decrypt/Mp4Decrypt.cpp

*/

#include "mp4decrypt.h"
#include "Ap4.h" // IWYU pragma: keep
#include <cstdlib>
#include <cstring>

int ap4_mp4decrypt(const unsigned char data[], unsigned int data_size,
                   const unsigned char *keys, unsigned int keys_count,
                   unsigned char **out_data, unsigned int *out_size) {
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

  delete processor;
  input->Release();

  if (AP4_FAILED(result)) {
    output->Release();
    return result;
  }

  // Allocate and copy output for Rust to own
  *out_size = static_cast<unsigned int>(output->GetDataSize());
  *out_data = static_cast<unsigned char *>(malloc(*out_size));
  memcpy(*out_data, output->GetData(), *out_size);

  output->Release();
  return 0;
}

void ap4_free(unsigned char *ptr) { free(ptr); }
