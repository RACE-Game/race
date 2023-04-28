# Transactor and Validator

In RACE Protocol, games are served by a server cluster.  Each server runs in either Transactor mode or Validator mode.  Usually, the transactor is the first server that joins the game. It acts as a relay for other servers and clients, and makes all settlement transactions.  Other servers run in Validator mode, acting as validators and randomization participants.

The differences of both modes are listed below.

## Transactor Mode

- Calculating game state
- Making transaction for settlements
- Generating randomness
- Receiving client events
- Broadcasting events

![transactor](transactor.jpg)

## Validator Mode

- Calculating game state
- Voting inactive of the Transactor
- Generating randomness

![validator](validator.jpg)
