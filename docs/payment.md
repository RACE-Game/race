## Payment

For games with all its accounts on chain, the payment can be really complicated.  We introduce a concept of intermediate recipient to simplify this procedure.  An recipient account can be created to receive tokens, it can have one or more wallets behind, each owns a share, we call them receivers.  The newly added receiver must be approved by all the owners.  When assets paid to this receipient account, it will be splited into different orders, and each receiver can claim its own parts.

![Payment](payment.jpg)

This model is simple and flexible.  A complex payment can be
restructured in a simple way.  For example, following is a tournament
game, with a crowedfunding prize pool and multiple sponsors, each
sponsor wants some share from the game commission.

![Tournament Payment](tournament-payment.jpg)

In this structure, the sponsor can contribute the prize pool.  The
contract can manage the payment structure since the tournament is
considered as the owner.
