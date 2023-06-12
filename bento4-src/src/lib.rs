//! Source of [bento4](https://github.com/axiomatic-systems/Bento4) and logic to build it.
//! See this [file](https://github.com/clitic/vsd/blob/main/mp4decrypt/build.rs) for example usage.

pub use cc;

use std::{env, path::PathBuf};

pub fn version() -> String {
    "1.6.0-640".to_owned()
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
    println!(
        "cargo:rerun-if-changed={}",
        root.join("Bento4").to_string_lossy()
    );

    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .extra_warnings(false)
        .includes(includes())
        .files([
            root.join("Bento4/Source/C++/Codecs/Ap4Ac3Parser.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4Ac4Parser.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4AdtsParser.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4AvcParser.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4BitStream.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4Eac3Parser.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4HevcParser.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4Mp4AudioInfo.cpp"),
            root.join("Bento4/Source/C++/Codecs/Ap4NalParser.cpp"),
        ])
        .files([
            root.join("Bento4/Source/C++/Core/Ap4.cpp"),
            root.join("Bento4/Source/C++/Core/Ap48bdlAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Ac4Utils.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4AinfAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Atom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4AtomFactory.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4AtomSampleTable.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Av1cAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4AvccAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4BlocAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4ByteStream.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Co64Atom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Command.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4CommandFactory.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4CommonEncryption.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4ContainerAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4CttsAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Dac3Atom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Dac4Atom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4DataBuffer.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Debug.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Dec3Atom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4DecoderConfigDescriptor.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4DecoderSpecificInfoDescriptor.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Descriptor.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4DescriptorFactory.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4DrefAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4DvccAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4ElstAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4EsDescriptor.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4EsdsAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Expandable.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4File.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4FileCopier.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4FileWriter.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4FragmentSampleTable.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4FrmaAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4FtypAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4GrpiAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4HdlrAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4HintTrackReader.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4HmhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4HvccAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4IkmsAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4IodsAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Ipmp.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4IproAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4IsfmAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4IsltAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4IsmaCryp.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4LinearReader.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Marlin.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4MdhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4MehdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4MfhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4MfroAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4MoovAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Movie.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4MovieFragment.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Mpeg2Ts.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4MvhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4NmhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4ObjectDescriptor.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4OdafAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4OddaAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4OdheAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4OhdrAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4OmaDcf.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4PdinAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Piff.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Processor.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Protection.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4PsshAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Results.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4RtpAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4RtpHint.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SaioAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SaizAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Sample.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SampleDescription.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SampleEntry.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SampleSource.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SampleTable.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SbgpAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SchmAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SdpAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SegmentBuilder.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SencAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SgpdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SidxAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SLConfigDescriptor.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SmhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4StcoAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SthdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4String.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4StscAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4StsdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4StssAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4StszAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SttsAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Stz2Atom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4SyntheticSampleTable.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TencAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TfdtAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TfhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TfraAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TimsAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TkhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Track.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TrakAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TrefTypeAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TrexAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4TrunAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4UrlAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4Utils.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4UuidAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4VmhdAtom.cpp"),
            root.join("Bento4/Source/C++/Core/Ap4VpccAtom.cpp"),
        ])
        .files([
            root.join("Bento4/Source/C++/Crypto/Ap4AesBlockCipher.cpp"),
            root.join("Bento4/Source/C++/Crypto/Ap4Hmac.cpp"),
            root.join("Bento4/Source/C++/Crypto/Ap4KeyWrap.cpp"),
            root.join("Bento4/Source/C++/Crypto/Ap4StreamCipher.cpp"),
        ])
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
