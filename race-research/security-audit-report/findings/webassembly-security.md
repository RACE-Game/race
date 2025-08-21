# WebAssembly Security

This section analyzes the WASM game bundles used in Race Protocol for potential vulnerabilities, focusing on memory safety issues and malicious code injection. Additionally, the audit assesses the security of the WASM runtime environment and its isolation from the client application.

**Findings:**

{% hint style="warning" %}
This section of report is currently only accessible to the core team for security reasons. \
Once the identified issues are resolved they will be publicly revealed.
{% endhint %}

**Prioritization:**

* Addressing the high-severity finding related to potential memory safety vulnerabilities should be prioritized as it poses the most significant threat to the security of the client application.
* Implementing additional isolation measures for the WASM runtime environment is also important for mitigating the risk of malicious code injection and compromise.
* Formal verification of game bundles, while valuable, can be a time-consuming and expensive process. It should be considered a long-term goal for enhancing the security and reliability of the game logic.
