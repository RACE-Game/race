# Other Examples

In addition to the Draw Card and Raffle games, Race Protocol offers several other example implementations that showcase different functionalities and game mechanics:

* **Minimal Game**: This is the most basic example, demonstrating the minimal requirements for building a game handler and interacting with the Race API. It features a simple counter that can be incremented by players, illustrating basic event handling and state management.
* **Simple Settle Game**: This example focuses on the settlement process. It triggers a settlement transaction whenever there are more than two players in the game, distributing all tokens to the first player and ejecting everyone else. This showcases the use of the Settle instruction and how to handle asset distribution in a game.
* **Roshambo Game**: This example implements the classic Rock Paper Scissors game and demonstrates the use of immutable decisions. Players make hidden choices, and the game handler reveals them simultaneously to determine the winner. This showcases how Race Protocol can be used to manage hidden information and ensure fair play in games with simultaneous decision-making.
* **Chat Example**: This example demonstrates how to implement a basic chat feature within a Race game. Players can send public messages that are broadcasted to all participants, showcasing the use of the Message event and the broadcasting capabilities of the Transactor server.

These examples, along with the Draw Card and Raffle games, provide a diverse set of use cases for Race Protocol and illustrate how its features can be applied to build various types of blockchain games with different mechanics and functionalities. Developers can explore these examples to learn best practices and gain inspiration for building their own games on the Race Protocol infrastructure.

\
