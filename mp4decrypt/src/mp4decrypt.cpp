/*
    REFERENCES
    ----------

    1. https://github.com/axiomatic-systems/Bento4/blob/991d890908fb5da73704920c6de43480fb29f76a/Source/C%2B%2B/Apps/Mp4Decrypt/Mp4Decrypt.cpp

*/

#include <stdio.h>
#include <stdlib.h>

#include "Ap4.h"
#include "mp4decrypt.h"

int ap4_mp4decrypt(
    const unsigned char data[],
    unsigned int data_size,
    const char *keys[],
    unsigned int keys_size,
    void *decrypted_data,
    callback_rust callback
) {
    // create a key map object to hold keys
    AP4_ProtectionKeyMap key_map;

    for (int i = 0; i < keys_size; i++)
    {
        char* keyid_text = NULL;
        char* key_text = NULL;

        if (AP4_SplitArgs(const_cast<char*>(keys[i]), keyid_text, key_text)) {
            return -999;
        }
        
        unsigned char key[16];
        unsigned int  track_id = 0;
        unsigned char kid[16];

        if (strlen(keyid_text) == 32) {
            if (AP4_ParseHex(keyid_text, kid, 16)) {
                return -998;
            }
        } else {
            track_id = (unsigned int)strtoul(keyid_text, NULL, 10);
            if (track_id == 0) {
                return -997;
            }
        }
        if (AP4_ParseHex(key_text, key, 16)) {
            return -996;
        }
        // set the key in the map
        if (track_id) {
            key_map.SetKey(track_id, key, 16);
        } else {
            key_map.SetKeyForKid(kid, key, 16);
        }
    }

    AP4_ByteStream* input = new AP4_MemoryByteStream(data, data_size);

    // create the decrypting processor
    AP4_Processor* processor = NULL;
    AP4_File* input_file = new AP4_File(*input);
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

    AP4_Result result;
    AP4_MemoryByteStream* output = new AP4_MemoryByteStream();
    
    // process/decrypt the file
    result = processor->Process(*input, *output, NULL);
    
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
