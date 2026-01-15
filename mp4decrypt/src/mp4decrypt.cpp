/*
    REFERENCES
    ----------

    1.
   https://github.com/axiomatic-systems/Bento4/blob/991d890908fb5da73704920c6de43480fb29f76a/Source/C%2B%2B/Apps/Mp4Decrypt/Mp4Decrypt.cpp

*/

#include "mp4decrypt.h"
#include "Ap4.h"
#include <cstdlib>
#include <cstring>

struct Ap4CencDecryptingProcessor {
  AP4_ProtectionKeyMap key_map;
  AP4_CencDecryptingProcessor *processor;
};

Ap4CencDecryptingProcessor *ap4_processor_new(const unsigned char *keys,
                                              unsigned int size) {
  Ap4CencDecryptingProcessor *ctx = new Ap4CencDecryptingProcessor();

  for (unsigned int i = 0; i < size; i++) {
    const unsigned char *kid = keys + (i * 32);
    const unsigned char *key = keys + (i * 32) + 16;
    ctx->key_map.SetKeyForKid(kid, key, 16);
  }

  ctx->processor = new AP4_CencDecryptingProcessor(&ctx->key_map);
  return ctx;
}

void ap4_processor_free(Ap4CencDecryptingProcessor *ctx) {
  delete ctx->processor;
  delete ctx;
}

void ap4_free(unsigned char *ptr) { free(ptr); }

int ap4_decrypt_file(Ap4CencDecryptingProcessor *ctx, const char *input_path,
                     const char *output_path, const char *init_path) {
  AP4_Result result;

  AP4_ByteStream *init = NULL;
  if (init_path != NULL) {
    result = AP4_FileByteStream::Create(
        init_path, AP4_FileByteStream::STREAM_MODE_READ, init);
    if (AP4_FAILED(result)) {
      return result;
    }
  }

  AP4_ByteStream *input = NULL;
  result = AP4_FileByteStream::Create(
      input_path, AP4_FileByteStream::STREAM_MODE_READ, input);
  if (AP4_FAILED(result)) {
    if (init) {
      init->Release();
    }
    return result;
  }

  AP4_ByteStream *output = NULL;
  result = AP4_FileByteStream::Create(
      output_path, AP4_FileByteStream::STREAM_MODE_WRITE, output);
  if (AP4_FAILED(result)) {
    if (init) {
      init->Release();
    }
    input->Release();
    return result;
  }

  if (init) {
    result = ctx->processor->Process(*input, *output, *init, NULL);
    init->Release();
  } else {
    result = ctx->processor->Process(*input, *output, NULL);
  }

  input->Release();
  output->Release();

  return result;
}

int ap4_decrypt_memory(Ap4CencDecryptingProcessor *ctx,
                       const unsigned char *input_data, unsigned int input_size,
                       unsigned char **output_data, unsigned int *output_size) {
  AP4_ByteStream *input = new AP4_MemoryByteStream(input_data, input_size);
  AP4_MemoryByteStream *output = new AP4_MemoryByteStream();
  AP4_Result result = ctx->processor->Process(*input, *output, NULL);

  input->Release();

  if (AP4_FAILED(result)) {
    output->Release();
    return result;
  }

  *output_size = static_cast<unsigned int>(output->GetDataSize());
  *output_data = static_cast<unsigned char *>(malloc(*output_size));

  if (*output_data == NULL) {
    output->Release();
    return -1;
  }

  memcpy(*output_data, output->GetData(), *output_size);
  output->Release();

  return 0;
}
