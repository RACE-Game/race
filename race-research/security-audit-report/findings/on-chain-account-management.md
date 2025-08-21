# On-chain Account Management

This section reviews the security of on-chain account creation, access control, and management processes in Race Protocol. It also assesses the design and implementation of game accounts, game bundle accounts, player profiles, and registration accounts.

**Findings:**

{% hint style="warning" %}
This section of report is currently only accessible to the core team for security reasons. \
Once the identified issues are resolved they will be publicly revealed.
{% endhint %}

**Design and Implementation Assessment:**

* **Game Accounts:** The design of game accounts appears to be comprehensive and covers the necessary data elements for managing game state, players, servers, and settlements. However, the lack of on-chain verification for game state updates is a significant security concern that needs to be addressed.
* **Game Bundle Accounts:** The use of NFTs to represent game bundles is a suitable approach for establishing ownership and linking to off-chain WASM data. However, the security of the decentralized storage solution used to store the WASM bundles should be carefully evaluated.
* **Player Profiles:** Player profiles provide a convenient way to manage player information and assets. However, additional security measures, such as multi-factor authentication or transaction confirmation prompts, could be implemented to further protect player accounts from unauthorized access.
* **Registration Accounts:** The concept of public and private registration accounts offers flexibility for developers and platform operators. However, the access control mechanisms for private registrations need to be strengthened to prevent unauthorized manipulation.
