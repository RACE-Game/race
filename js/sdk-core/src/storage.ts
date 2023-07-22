export interface IStorage {
    getItem(key: string): string | null;
    setItem(key: string, value: any): void;
}
