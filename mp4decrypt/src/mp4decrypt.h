#ifndef __MP4_DECRYPT_H__
#define __MP4_DECRYPT_H__

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Ap4CencDecryptingProcessor Ap4CencDecryptingProcessor;

Ap4CencDecryptingProcessor *ap4_processor_new(const unsigned char *keys,
                                              unsigned int size);

void ap4_processor_free(Ap4CencDecryptingProcessor *ctx);

void ap4_free(unsigned char *ptr);

int ap4_decrypt_file(Ap4CencDecryptingProcessor *ctx, const char *input_path,
                     const char *output_path, const char *init_path);

int ap4_decrypt_memory(Ap4CencDecryptingProcessor *ctx,
                       const unsigned char *input_data, unsigned int input_size,
                       unsigned char **output_data, unsigned int *output_size);

#ifdef __cplusplus
}
#endif

#endif
