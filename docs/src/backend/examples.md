# Examples

In this sections some examples of backend applications are described.

## Hello-world

[Hello-world](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world) is a simple demo app for the Fluence network that shows a basic usage of the Fluence SDK.

## Hello-world2 with logging

[Hello-world2](https://github.com/fluencelabs/fluence/tree/master/vm/examples/hello-world2/app-2018) is a simple demo app for the Fluence network that shows either a basic usage and the logger feature of the Fluence SDK.

## Llamadb

[Llamadb](https://github.com/fluencelabs/fluence/tree/master/vm/examples/llamadb) app is an example of adopting of existing in-memory SQL database to the Fluence network. This app is based on the original [llamadb](https://github.com/fluencelabs/llamadb) database. Since it is in-memory database and doesn't use multithreading, disk storage or other OS features it can be easily ported to the Fluence. All that need to do it is a simple [wrapper](https://github.com/fluencelabs/fluence/blob/master/vm/examples/llamadb/src/lib.rs) that receives requests from `client-side`, manages it to the original `llamadb` and returns results back.

## Tic-tac-toe

[tic-tac-toe](https://github.com/fluencelabs/fluence/tree/master/vm/examples/tic-tac-toe) app is an example of proper memory management and function routing scheme through json. This app provides us with a bunch of public methods:

- `create_payer` - creates a new player with a given player name
    
- `create_game` - creates a new game for a provided player
    
- `move` - makes user and app moves sequentially, returns app move coordinates 
    
- `get_game_state` - returns state of game for given user (includes board state and user tile)
    
- `get_statistics` - retuns overall game statistics includes how registred players count, created game and overall moves count

### Function routing scheme

Since there are some public api functions that receives different parameters it needs a way to choose one of them with appropriate parameters. There are several input data format that could be used to reach our goal, but we chose JSON as a one of the most popular that uses text to transmit data objects consisting of key-value pairs.

On the other side one of the most popular json parser for Rust is a [serde](https://github.com/serde-rs/serde) library. Serde provides `serde_json::from_str` method that parses string to a set of `serde_json::Value` enums and `serde_json::from_value` method that can translate `Value` to a concrete struct. It is very useful to parsing json that have the same part.

Let's review it usage on a example of `do_request` function realization:

```Rust
fn do_request(req: String) -> AppResult<Value> {
    let raw_request: Value = serde_json::from_str(req.as_str())?;
    let request: Request = serde_json::from_value(raw_request.clone())?;

    match request.action.as_str() {
        "move" => {
            let player_move: PlayerMove = serde_json::from_value(raw_request)?;
            GAME_MANAGER.with(|gm| {
                gm.borrow()
                    .make_move(request.player_name, player_move.coords)
            })
        }
        ...
        "create_game" => {
            let player_tile: PlayerTile = serde_json::from_value(raw_request)?;
            let player_tile = game::Tile::from_char(player_tile.tile).ok_or_else(|| {
                "incorrect tile type, please choose it from {'X', 'O'} set".to_owned()
            })?;

            GAME_MANAGER.with(|gm| {
                gm.borrow_mut()
                    .create_game(request.player_name, player_tile)
            })
        }

}
```

It can be viewed that at first `req` is parsed to `serde_json::Value` (and this parsing is occurred only once), then the result is parsed to a `Request` struct same for all requests. This struct contains `action` and `player_name` fields. After this parsing routing could be done by `action` field value. Each action can has it own request structure and additional parsing to a concrete structure can be done in the same way (please see `move` and `create_game` cases from the above example).

### Memory management

Proper memory management is a corner stone of application that provides capabilities to store some user-supplied data. In tic-tac-toe it is solved by using a fixed-size collection for all internal objects that can be of two types: `Player` and `Game`. `Player` contains a string that represents its name and `Game` contains board state and a tile chosen by user.

Since `create_player` allows to create an infinite count of `Player`, they could exhaust all app memory. [ArrayDeque](https://github.com/andylokandy/arraydeque) manages to solve this problem by providing a fixed size queue that can pop up object from the front while exceeding the limit.

Then consider a situation when each user can have play to several games or the size of `Game` is too big compare to the size of `Player` to give example that is more close to real applications. In this situation one of the approaches is use a separate queue for `Game` objects (of cause `Game` can be saved into a `Player` even in vector or map but consider a more complex design that give a more control of memory management). To implement it each `Player` should has a smth like id of a proper `Game` object. Since all of them are placed in queue that can change it position in some time, the only way is using a smart pointers. It this example there are two queues that own `Player` and `Game` objects (it means that they store a strong Rc ptr to them) and `Player` has a weak ptr to a proper `Game` object.

Also it would be good to find `Player` object by name more faster than `O(n)`. To reach it `HashMap` can be used. And because of our design `HashMap` should also contains weak pointers to players. According to all of these the final structure could look like this: 

```Rust
pub struct GameManager {
    players: ArrayDeque<[Rc<RefCell<Player>>; PLAYERS_MAX_COUNT], Wrapping>,
    games: ArrayDeque<[Rc<RefCell<Game>>; GAMES_MAX_COUNT], Wrapping>,
    players_by_name: HashMap<String, Weak<RefCell<Player>>>,
}
```

### Authentication

TBD