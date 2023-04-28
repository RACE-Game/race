# Randomization

In a strategy game, one way the hidden knowledge is created is through randomization.  For example, in card games, players start with different hands.  In Race, the games are served by general servers and the mental poker algorithm is adopted for randomization.  The randomization can be done either by players or servers.  Usually player-generated randomness represents better fairness, but it works poorly in bad network connection, which is pretty common in reality.  Server-generated randomization, on the other hand, is fast but servers from different owners are required for securing fairness.

Some additional means can be introduced to increase the fairness. For example, Servers generate randomness but players pick the items on their own.

## Implementation

The whole progress is described as below:

![Randomization](random.jpg)

Currently, Chacha20 is used as item secrets and RSA is used for the encryption of private communication.

TODO
-[ ] Add resource URLs of Chacha20 and RSA
