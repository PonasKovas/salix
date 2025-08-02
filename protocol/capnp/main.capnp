@0x8a655ccffe23e9a2;

interface Handshake {
    struct Version {
        major @0: UInt32;
        minor @1: UInt32;
        patch @2: UInt32;
    }

    handshake @0 (client_version: Version) -> (server_version: Version, main: Main);
}

interface Main {
    struct HelloRequest {
        name @0 :Text;
    }

    struct HelloReply {
        message @0 :Text;
    }

    sayHello @0 (request: HelloRequest) -> (reply: HelloReply);
}