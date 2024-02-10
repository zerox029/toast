struct Superblock {

}

enum FileSystemState {
    Clean = 1,
    Error = 2,
}

enum ErrorHandlingMethod {
    Ignore = 1,
    RemountReadOnly = 2,
    KernelPanic = 3,
}

enum CreatorOSId {
    Linux = 0,
    GnuHurd = 1,
    Masix = 2,
    FreeBSD = 3,
    OtherLite = 4,
}