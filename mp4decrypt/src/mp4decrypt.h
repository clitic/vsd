#ifndef __MP4_DECRYPT_H__
#define __MP4_DECRYPT_H__

#ifdef __cplusplus
extern "C" {
#endif

int ap4_mp4decrypt(const unsigned char data[], unsigned int data_size,
                   const unsigned char *keys, unsigned int keys_count,
                   unsigned char **out_data, unsigned int *out_size);

void ap4_free(unsigned char *ptr);

#ifdef __cplusplus
}
#endif

#endif
