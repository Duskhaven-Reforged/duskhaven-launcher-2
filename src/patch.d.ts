export interface Patch {
    ArrayNumber: number;
    Checksum: string;
    ContentType: string;
    DateCreated: string;
    Guid: string;
    IsDirectory: boolean;
    LastChanged: string;
    Length: number;
    ObjectName: string;
    Path: string;
    ReplicatedZones: string;
    ServerId: number;
    StorageZoneId: number;
    StorageZoneName: string;
    UserId: string;
    filePath?: string;
}

export interface Progress {
    download_id: number,
    filesize: number,
    transfered: number,
    transfer_rate: number,
    percentage: number,
}