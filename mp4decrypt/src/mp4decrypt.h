#ifndef __MP4_DECRYPT_H__
#define __MP4_DECRYPT_H__

#ifdef __cplusplus
extern "C"
{
#endif

    typedef void (*rust_store_callback)(void *, const unsigned char *data, unsigned int length);
    int decrypt_in_memory(
        const unsigned char data[],
        unsigned int data_size,
        const char* keyids[],
        const char* keys[],
        int nkeys,
        void* decrypted_data,
        rust_store_callback callback
    );
    int decrypt_in_memory_with_fragments_info(
        const unsigned char data[],
        unsigned int data_size,
        const char* keyids[],
        const char* keys[],
        int nkeys,
        void* decrypted_data,
        rust_store_callback callback,
        const unsigned char fragments_info_data[],
        unsigned int fragments_info_data_size
    );

#ifdef __cplusplus
}
#endif

#endif /* __MP4_DECRYPT_H__ */
