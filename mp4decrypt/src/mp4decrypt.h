#ifndef __MP4_DECRYPT_H__
#define __MP4_DECRYPT_H__

#ifdef __cplusplus
extern "C"
{
#endif

    typedef void (*callback_rust)(void *, const unsigned char *data, unsigned int length);

    int ap4_mp4decrypt(
        const unsigned char data[],
        unsigned int data_size,
        const char *keys[],
        unsigned int keys_size,
        void *decrypted_data,
        callback_rust callback);

#ifdef __cplusplus
}
#endif

#endif /* __MP4_DECRYPT_H__ */
