const importObject = {
  module: {},
  env: {
    memory: new WebAssembly.Memory({ initial: 1024 })
  }
}

window.addEventListener("message", (event) => {
  if (event.origin != "http://localhost:8000") return;
  console.log(event);
})

import("./pkg").then(module => {
}).catch(err => {
  console.error(err)
})
