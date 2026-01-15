#ifndef __MP4_DECRYPT_H__
#define __MP4_DECRYPT_H__

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to decryption context
typedef struct Ap4Context Ap4Context;

// Create a decryption context with the given keys
// keys: flat array of [kid1][key1][kid2][key2]... (32 bytes per pair)
Ap4Context *ap4_context_new(const unsigned char *keys, unsigned int keys_count);

// Decrypt data using an existing context (reusable)
int ap4_decrypt(Ap4Context *ctx, const unsigned char *data,
                unsigned int data_size, unsigned char **out_data,
                unsigned int *out_size);

// Decrypt file using streaming I/O (no memory limit)
// init_path: optional init segment path (NULL if input contains init)
int ap4_decrypt_file(Ap4Context *ctx, const char *init_path,
                     const char *input_path, const char *output_path);

// Free the decryption context
void ap4_context_free(Ap4Context *ctx);

// Free data returned by ap4_decrypt
void ap4_free(unsigned char *ptr);

#ifdef __cplusplus
}
#endif

#endif
