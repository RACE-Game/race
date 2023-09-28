export interface IStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: any): void;
}


export type TtlCache<T> = {
  expire: number;
  value: T;
};

/**
 * Set a cache with a `key`, and expire after `ttl` miliseconds.
 */
export function setTtlCache(storage: IStorage, key: string, value: any, ttl: number) {
  const data = {
    expire: new Date().getTime() + ttl,
    value
  };

  storage.setItem(key, JSON.stringify(data));
}


/**
 * Get a TTL cache value by `key`.
 */
export function getTtlCache<T>(storage: IStorage, key: string): T | undefined {
  const s = storage.getItem(key);
  if (!!s) {
    const data: TtlCache<T> = JSON.parse(s);
    if (data.expire < new Date().getTime()) {
      return undefined;
    }
    return data.value;
  }
  return undefined;
}
