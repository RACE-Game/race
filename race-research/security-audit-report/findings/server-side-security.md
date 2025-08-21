# Server-side Security

This section analyzes the Transactor and validator server implementations for potential vulnerabilities in communication, data handling, and the P2P randomization process. It also reviews server-side security measures for protecting game state and player assets.

**Findings:**

{% hint style="warning" %}
This section of report is currently only accessible to the core team for security reasons. \
Once the identified issues are resolved they will be publicly revealed.
{% endhint %}

**P2P Randomization Security:**

* The P2P randomization process appears to be designed securely, utilizing a variant of the mental poker algorithm.
* However, further analysis and testing are recommended to ensure its resistance to manipulation and collusion attacks by potentially malicious servers.

**Server-side Security Measures:**

* The codebase does not explicitly show robust server-side security measures for protecting game state and player assets.
* It is recommended to implement additional security practices, such as secure storage of secret keys, access control mechanisms, and intrusion detection systems, to further protect the server environment and sensitive data.
