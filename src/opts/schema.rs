use clap::Clap;
use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames};

/// Binding to a given address and listen for requests
#[derive(Clap, Debug)]
pub struct SchemaCommand {
    #[clap(subcommand)]
    pub command: SchemaSubcommand,
}

#[derive(Clap, Debug)]
pub enum SchemaSubcommand {
    /// Lists all possible items to print schema
    #[clap(name = "list")]
    List,

    /// Prints information about schema for specific item
    #[clap(name = "info")]
    Info(SchemaInfo),
}

#[derive(Clap, Debug)]
pub struct SchemaInfo {
    /// The type of message whose schema to print
    #[clap(
        name = "type",
        parse(try_from_str), 
        possible_values = &SchemaType::VARIANTS, 
    )]
    pub schema_type: SchemaType,
}

#[derive(Clap, Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum SchemaType {
    HeartbeatRequest,
    VersionRequest,
    CapabilitiesRequest,
    CreateDirRequest,
    RenameDirRequest,
    RemoveDirRequest,
    ListDirContentsRequest,
    OpenFileRequest,
    CloseFileRequest,
    RenameUnopenedFileRequest,
    RenameFileRequest,
    RemoveUnopenedFileRequest,
    RemoveFileRequest,
    ReadFileRequest,
    WriteFileRequest,
    ExecProcRequest,
    WriteProcStdinRequest,
    ReadProcStdoutRequest,
    ReadProcStderrRequest,
    KillProcRequest,
    ReadProcStatusRequest,
    SequenceRequest,
    BatchRequest,
    ForwardRequest,
    CustomRequest,
    InternalDebugRequest,
    HeartbeatReply,
    VersionReply,
    CapabilitiesReply,
    DirCreatedReply,
    DirRenamedReply,
    DirRemovedReply,
    DirContentsListReply,
    FileOpenedReply,
    FileClosedReply,
    UnopenedFileRenamedReply,
    FileRenamedReply,
    UnopenedFileRemovedReply,
    FileRemovedReply,
    FileContentsReply,
    FileWrittenReply,
    ProcStartedReply,
    ProcStdinWrittenReply,
    ProcStdoutContentsReply,
    ProcStderrContentsReply,
    ProcKilledReply,
    ProcStatusReply,
    ErrorReply,
    SequenceReply,
    BatchReply,
    ForwardReply,
    CustomReply,
    InternalDebugReply,
}
