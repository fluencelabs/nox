# Examples

## Llamadb

[Llamadb](https://github.com/fluencelabs/fluence/tree/master/vm/examples/llamadb) app is an example of adopting existing project in-memory SQL database to the Fluence network. This app is based on the original [llamadb](https://github.com/fluencelabs/llamadb) database. Since it is in-memory database and doesn't has multithreading, disk storage or other OS features it can be easily ported to the Fluence. All that need to be done for it is a simple wrapper that receives requests from `client-side`, manages it to the original `llamadb` and returns results back.



## Tic-tac-toe

[tic-tac-toe](https://github.com/fluencelabs/fluence/tree/master/vm/examples/tic-tac-toe)
