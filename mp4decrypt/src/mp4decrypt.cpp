/*
    REFERENCES
    ----------

    1. https://github.com/axiomatic-systems/Bento4/blob/991d890908fb5da73704920c6de43480fb29f76a/Source/C%2B%2B/Apps/Mp4Decrypt/Mp4Decrypt.cpp

*/

#include <stdio.h>
#include <stdlib.h>

#include "Ap4.h"
#include "mp4decrypt.h"

int decrypt_in_memory(
    const unsigned char data[],
    unsigned int data_size,
    const char* keyids[],
    const char* keys[],
    int nkeys,
    void* decrypted_data,
    rust_store_callback callback
) {
    // create a key map object to hold keys
    AP4_ProtectionKeyMap key_map;

    for (int i = 0; i < nkeys; i++)
    {
        unsigned char key[16];
        unsigned int  track_id = 0;
        unsigned char kid[16];

        if (strlen(keyids[i]) == 32) {
            if (AP4_ParseHex(keyids[i], kid, 16)) {
                return 100;
            }
        } else {
            track_id = (unsigned int)strtoul(keyids[i], NULL, 10);
            if (track_id == 0) {
                return 101;
            }
        }
        if (AP4_ParseHex(keys[i], key, 16)) {
            return 102;
        }
        // set the key in the map
        if (track_id) {
            key_map.SetKey(track_id, key, 16);
        } else {
            key_map.SetKeyForKid(kid, key, 16);
        }
    }

    AP4_MemoryByteStream* input = new AP4_MemoryByteStream(data, data_size);

    // create the decrypting processor
    AP4_Processor* processor = NULL;
    AP4_File* input_file = new AP4_File(*input);
    // input_file->SetFileType()
    AP4_FtypAtom* ftyp = input_file->GetFileType();
    if (ftyp) {
        if (ftyp->GetMajorBrand() == AP4_OMA_DCF_BRAND_ODCF || ftyp->HasCompatibleBrand(AP4_OMA_DCF_BRAND_ODCF)) {
            processor = new AP4_OmaDcfDecryptingProcessor(&key_map);
        } else if (ftyp->GetMajorBrand() == AP4_MARLIN_BRAND_MGSV || ftyp->HasCompatibleBrand(AP4_MARLIN_BRAND_MGSV)) {
            processor = new AP4_MarlinIpmpDecryptingProcessor(&key_map);
        } else if (ftyp->GetMajorBrand() == AP4_PIFF_BRAND || ftyp->HasCompatibleBrand(AP4_PIFF_BRAND)) {
            processor = new AP4_CencDecryptingProcessor(&key_map);
        }
    }
    if (processor == NULL) {
        // no ftyp, look at the sample description of the tracks first
        AP4_Movie* movie = input_file->GetMovie();
        if (movie) {
            AP4_List<AP4_Track>& tracks = movie->GetTracks();
            for (unsigned int i=0; i<tracks.ItemCount(); i++) {
                AP4_Track* track = NULL;
                tracks.Get(i, track);
                if (track) {
                    AP4_SampleDescription* sdesc = track->GetSampleDescription(0);
                    if (sdesc && sdesc->GetType() == AP4_SampleDescription::TYPE_PROTECTED) {
                        AP4_ProtectedSampleDescription* psdesc = AP4_DYNAMIC_CAST(AP4_ProtectedSampleDescription, sdesc);
                        if (psdesc) {
                            if (psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CENC ||
                                psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CBC1 ||
                                psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CENS ||
                                psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CBCS) {
                                processor = new AP4_CencDecryptingProcessor(&key_map);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // by default, try a standard decrypting processor
    if (processor == NULL) {
        processor = new AP4_StandardDecryptingProcessor(&key_map);
    }
    
    delete input_file;
    input_file = NULL;
    input->Seek(0);

    AP4_MemoryByteStream* output = new AP4_MemoryByteStream();
    AP4_AtomFactory atom_factory;
    AP4_Result result = processor->Process(*input, *output, NULL, atom_factory);
    if (AP4_FAILED(result)) {
        return result;
    }

    // cleanup
    delete processor;
    input->Release();
    callback(decrypted_data, output->GetData(), output->GetDataSize());
    output->Release();
    return 0;
}

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
) {
    // create a key map object to hold keys
    AP4_ProtectionKeyMap key_map;

    for (int i = 0; i < nkeys; i++)
    {
        unsigned char key[16];
        unsigned int  track_id = 0;
        unsigned char kid[16];

        if (strlen(keyids[i]) == 32) {
            if (AP4_ParseHex(keyids[i], kid, 16)) {
                return 100;
            }
        } else {
            track_id = (unsigned int)strtoul(keyids[i], NULL, 10);
            if (track_id == 0) {
                return 101;
            }
        }
        if (AP4_ParseHex(keys[i], key, 16)) {
            return 102;
        }
        // set the key in the map
        if (track_id) {
            key_map.SetKey(track_id, key, 16);
        } else {
            key_map.SetKeyForKid(kid, key, 16);
        }
    }

    AP4_MemoryByteStream* fragments_info = new AP4_MemoryByteStream(fragments_info_data, fragments_info_data_size);

    // create the decrypting processor
    AP4_Processor* processor = NULL;
    AP4_File* input_file = new AP4_File(*fragments_info);
    // input_file->SetFileType()
    AP4_FtypAtom* ftyp = input_file->GetFileType();
    if (ftyp) {
        if (ftyp->GetMajorBrand() == AP4_OMA_DCF_BRAND_ODCF || ftyp->HasCompatibleBrand(AP4_OMA_DCF_BRAND_ODCF)) {
            processor = new AP4_OmaDcfDecryptingProcessor(&key_map);
        } else if (ftyp->GetMajorBrand() == AP4_MARLIN_BRAND_MGSV || ftyp->HasCompatibleBrand(AP4_MARLIN_BRAND_MGSV)) {
            processor = new AP4_MarlinIpmpDecryptingProcessor(&key_map);
        } else if (ftyp->GetMajorBrand() == AP4_PIFF_BRAND || ftyp->HasCompatibleBrand(AP4_PIFF_BRAND)) {
            processor = new AP4_CencDecryptingProcessor(&key_map);
        }
    }
    if (processor == NULL) {
        // no ftyp, look at the sample description of the tracks first
        AP4_Movie* movie = input_file->GetMovie();
        if (movie) {
            AP4_List<AP4_Track>& tracks = movie->GetTracks();
            for (unsigned int i=0; i<tracks.ItemCount(); i++) {
                AP4_Track* track = NULL;
                tracks.Get(i, track);
                if (track) {
                    AP4_SampleDescription* sdesc = track->GetSampleDescription(0);
                    if (sdesc && sdesc->GetType() == AP4_SampleDescription::TYPE_PROTECTED) {
                        AP4_ProtectedSampleDescription* psdesc = AP4_DYNAMIC_CAST(AP4_ProtectedSampleDescription, sdesc);
                        if (psdesc) {
                            if (psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CENC ||
                                psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CBC1 ||
                                psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CENS ||
                                psdesc->GetSchemeType() == AP4_PROTECTION_SCHEME_TYPE_CBCS) {
                                processor = new AP4_CencDecryptingProcessor(&key_map);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // by default, try a standard decrypting processor
    if (processor == NULL) {
        processor = new AP4_StandardDecryptingProcessor(&key_map);
    }
    
    delete input_file;
    input_file = NULL;
    fragments_info->Seek(0);

    AP4_MemoryByteStream* input = new AP4_MemoryByteStream(data, data_size);
    AP4_MemoryByteStream* output = new AP4_MemoryByteStream();
    AP4_AtomFactory atom_factory;
    AP4_Result result = processor->Process(*input, *output, *fragments_info, NULL, atom_factory);
    if (AP4_FAILED(result)) {
        return result;
    }

    // cleanup
    delete processor;
    input->Release();
    callback(decrypted_data, output->GetData(), output->GetDataSize());
    fragments_info->Release();
    output->Release();
    return 0;
}
