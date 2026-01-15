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

struct Ap4Context {
  AP4_ProtectionKeyMap key_map;
  AP4_CencDecryptingProcessor *processor;
};

Ap4Context *ap4_context_new(const unsigned char *keys,
                            unsigned int keys_count) {
  Ap4Context *ctx = new Ap4Context();

  for (unsigned int i = 0; i < keys_count; i++) {
    const unsigned char *kid = keys + (i * 32);
    const unsigned char *key = keys + (i * 32) + 16;
    ctx->key_map.SetKeyForKid(kid, key, 16);
  }

  ctx->processor = new AP4_CencDecryptingProcessor(&ctx->key_map);
  return ctx;
}

int ap4_decrypt(Ap4Context *ctx, const unsigned char *data,
                unsigned int data_size, unsigned char **out_data,
                unsigned int *out_size) {
  AP4_ByteStream *input = new AP4_MemoryByteStream(data, data_size);
  AP4_MemoryByteStream *output = new AP4_MemoryByteStream();
  AP4_Result result = ctx->processor->Process(*input, *output, NULL);

  input->Release();

  if (AP4_FAILED(result)) {
    output->Release();
    return result;
  }

  *out_size = static_cast<unsigned int>(output->GetDataSize());
  *out_data = static_cast<unsigned char *>(malloc(*out_size));

  if (*out_data == NULL) {
    output->Release();
    return -1;
  }

  memcpy(*out_data, output->GetData(), *out_size);
  output->Release();

  return 0;
}

void ap4_context_free(Ap4Context *ctx) {
  delete ctx->processor;
  delete ctx;
}

void ap4_free(unsigned char *ptr) { free(ptr); }
