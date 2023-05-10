//! Source of [bento4](https://github.com/axiomatic-systems/Bento4) and logic to build it.
//! See this [file](https://github.com/clitic/vsd/blob/main/mp4decrypt/build.rs) for example usage.
 
pub use cc;

use std::{env, path::PathBuf};

pub fn version() -> String {
    "1.6.0-639".to_owned()
}

pub fn includes() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    vec![
        root.join("Bento4/Source/C++/Core"),
        root.join("Bento4/Source/C++/Codecs"),
        root.join("Bento4/Source/C++/Crypto"),
        root.join("Bento4/Source/C++/MetaData"),
    ]
}

pub fn build() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    println!("cargo:rerun-if-changed={}", root.join("Bento4").to_string_lossy());

    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .extra_warnings(false)
        .includes(includes())
        .file(root.join("Bento4/Source/C++/Codecs/Ap4Ac3Parser.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4Ac4Parser.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4AdtsParser.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4AvcParser.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4BitStream.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4Eac3Parser.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4HevcParser.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4Mp4AudioInfo.cpp"))
        .file(root.join("Bento4/Source/C++/Codecs/Ap4NalParser.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap48bdlAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Ac4Utils.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4AinfAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Atom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4AtomFactory.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4AtomSampleTable.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Av1cAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4AvccAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4BlocAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4ByteStream.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Co64Atom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Command.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4CommandFactory.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4CommonEncryption.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4ContainerAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4CttsAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Dac3Atom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Dac4Atom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4DataBuffer.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Debug.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Dec3Atom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4DecoderConfigDescriptor.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4DecoderSpecificInfoDescriptor.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Descriptor.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4DescriptorFactory.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4DrefAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4DvccAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4ElstAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4EsDescriptor.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4EsdsAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Expandable.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4File.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4FileCopier.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4FileWriter.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4FragmentSampleTable.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4FrmaAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4FtypAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4GrpiAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4HdlrAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4HintTrackReader.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4HmhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4HvccAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4IkmsAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4IodsAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Ipmp.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4IproAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4IsfmAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4IsltAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4IsmaCryp.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4LinearReader.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Marlin.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4MdhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4MehdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4MfhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4MfroAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4MoovAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Movie.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4MovieFragment.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Mpeg2Ts.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4MvhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4NmhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4ObjectDescriptor.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4OdafAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4OddaAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4OdheAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4OhdrAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4OmaDcf.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4PdinAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Piff.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Processor.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Protection.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4PsshAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Results.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4RtpAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4RtpHint.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SaioAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SaizAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Sample.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SampleDescription.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SampleEntry.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SampleSource.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SampleTable.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SbgpAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SchmAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SdpAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SegmentBuilder.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SencAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SgpdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SidxAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SLConfigDescriptor.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SmhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4StcoAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SthdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4String.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4StscAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4StsdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4StssAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4StszAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SttsAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Stz2Atom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4SyntheticSampleTable.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TencAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TfdtAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TfhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TfraAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TimsAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TkhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Track.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TrakAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TrefTypeAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TrexAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4TrunAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4UrlAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4Utils.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4UuidAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4VmhdAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Core/Ap4VpccAtom.cpp"))
        .file(root.join("Bento4/Source/C++/Crypto/Ap4AesBlockCipher.cpp"))
        .file(root.join("Bento4/Source/C++/Crypto/Ap4Hmac.cpp"))
        .file(root.join("Bento4/Source/C++/Crypto/Ap4KeyWrap.cpp"))
        .file(root.join("Bento4/Source/C++/Crypto/Ap4StreamCipher.cpp"))
        .file(root.join("Bento4/Source/C++/MetaData/Ap4MetaData.cpp"))
        .file(root.join("Bento4/Source/C++/System/StdC/Ap4StdCFileByteStream.cpp"))
        .file(
            if env::var("CARGO_CFG_TARGET_OS")
                .expect("CARGO_CFG_TARGET_OS env variable not set by cargo?")
                == "windows"
            {
                root.join("Bento4/Source/C++/System/Win32/Ap4Win32Random.cpp")
            } else {
                root.join("Bento4/Source/C++/System/Posix/Ap4PosixRandom.cpp")
            },
        )
        .out_dir(
            env::var("OUT_DIR")
                .map(|x| PathBuf::from(x).join("bento4"))
                .expect("OUT_DIR env variable not set by cargo?"),
        )
        .compile("ap4");
}