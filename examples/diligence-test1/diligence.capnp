@0xef862f668d5adcb6;

using import "/schema/storage.capnp".Cell;
using Sqlite = import "/schema/sqlite.capnp";

# A Keystone module must expose an interface called Root that corresponds to the API returned by start()
interface Root {
    struct EchoRequest {
        name @0 :Text;
    }

    struct EchoReply {
        message @0 :Text;
    }

    echoAlphabetical @0 (request: EchoRequest) -> (reply: EchoReply);
    runServer @1 () -> ();
    stop @2 () -> ();
    captureSqliteRequests @3 () -> ();
}

# All modules must have a struct named "Config" that keystone can look up when compiling
# the root configuration file.
struct Config {
    inner @0 :Cell(Sqlite.Table);
    sqlite @1 :Sqlite.Root;
}
#const properties :ModuleProperties = (
#    friendlyName = "Hello Diligence",
#    stateful = true,
#    spawnID = PosixExecutable,
#    spawnDesc = (
#        path = "/usr/bin/diligence",
#        arch = Native
#    )
#);
