## Contract

The same instructions are implemented in different contracts for multi-chain support.

### The list of instructions


| Instruction         | Permission                                      | Typical Sender |
|---------------------|-------------------------------------------------|----------------|
| CreateGameAccount   | Everyone                                        | Host           |
| CloseGameAccount    | Game Owner                                      | Host           |
| CreateRegistry      | Everyone                                        | Player         |
| CreatePlayerProfile | Everyone                                        | Player         |
| Settle              | The Transactor of the game                      | Server         |
| Vote                | Everyone in the game(Currently all the servers) | Server         |
| ServeGame           | Everyone                                        | Server         |
| RegisterGame        | Everyone for public reg, owner for private reg  | Host           |
| UnregisterGame      | Game owner or reg owner                         | Host           |
| JoinGame            | Everyone                                        | Player         |
| PublishGame         | Everyone                                        | Developer      |
