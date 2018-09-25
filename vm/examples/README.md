
### Examples written in Rust for running with Fluence VM.

Check that you've installed `docker` and `docker` is running.

#### For compiling and running an example just invoke

        sbt vm-{EXAMPLE_NAME}/run 
        
from project root folder. This Task compiles Rust code to Wasm code with `docker` image,
builds fat jar for this example and runs it. For example:
    
        sbt vm-counter/run
        
A Task should finish with an error when something goes wrong.
If running is success you will see output like this:
    
    
    [info] [success] Total time: 4 s, completed 29.08.2018 10:59:42
    [info]     Run example counter.jar
    [info] Starts for input file /Users/${YOUR_HOME}/dev/fluence/dataengine/vm/examples/counter/target/wasm32-unknown-unknown/release/counter.wasm
    [info]           
    [info]         [SUCCESS] Execution Results.
    [info]         initState=ByteVector(32 bytes, 0x2840d82f33ef471c2204f92609b60401e0c5547e5b7648bc4804a68221cce7c7)
    [info]         get1=Some(1)
    [info]         get2=Some(3)
    [info]         finishState=ByteVector(32 bytes, 0x337754aaa2ea79f3f8069df05d2f3e73a37fe447e9410d1e781b7dc046a5a9f0)
    [info]       
    [success] Total time: 22 s, completed 29.08.2018 10:59:45
