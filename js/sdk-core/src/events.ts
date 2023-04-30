
export type PlayerJoin = {
  addr: string;
  position: number;
  balance: bigint;
  access_version: bigint;
};

export type ServerJoin = {
  addr: string;
  endpoint: string;
  access_version: bigint;
};

export type SecretShare = {

};

export type Event =
  {
    Custom: {
      sender: string;
      raw: string;
    }
  }
  | "Ready"
  | {
    ShareSecrets: {
      sender: string;
      shares: SecretShare[];
    }
  }
  | {
    OperationTimeout: {
      addrs: string[];
    }
  }
  | {
    Mask: {
      sender: string;
      random_id: bigint;
      ciphertexts: Uint8Array,
    }
  }
  | {
    Lock: {
      sender: string;
      random_id: bigint;
      ciphertexs_and_digests: Array<[Uint8Array, Uint8Array]>;
    }
  }
  | {
    RandomnessReady: {
      random_id: bigint,
    }
  }
  | {
    Sync: {
      new_players: PlayerJoin[],
      new_servers: ServerJoin[],
      transactor_addr: string;
      access_version: bigint,
    }
  }
  | {
    ServerLeave: {
      server_addr: string;
      transactor_addr: string;
    }
  }
  | {
    Leave: {
      player_addr: string;
    }
  }
  | {
    GameStart: {
      access_version: bigint;
    }
  }
  | "WaitingTimeout"
  | {
    DrawRandomItems: {
      sender: string;
      random_id: number;
      indexes: number[];
    }
  }
  | "DrawTimeout"
  | {
    ActionTimeout: {
      player_addr: string;
    }
  }
  | {
    AnswerDecision: {
      sender: string;
      decision_id: bigint;
      ciphertext: Uint8Array;
      digest: Uint8Array;
    }
  }
  | "SecretsReady"
  | "Shutdown";

export function makeCustomEvent(sender: string, customEvent: any): Event {
  return {
    Custom: {
      sender,
      raw: JSON.stringify(customEvent)
    }
  }
}
