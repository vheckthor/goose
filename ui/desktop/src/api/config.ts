// Generated API types will be in this directory
import { ConfigApi } from './generated';

export class Config {
  static async upsert(key: string, value: any, isSecret?: boolean) {
    return await ConfigApi.upsertConfig({ key, value, isSecret });
  }

  static async read(key: string) {
    return await ConfigApi.readConfig({ key });
  }

  static async remove(key: string) {
    return await ConfigApi.removeConfig({ key });
  }

  static async readAll() {
    return await ConfigApi.readAllConfig();
  }

  static async addExtension(name: string, config: any) {
    return await ConfigApi.addExtension({ name, config });
  }

  static async removeExtension(name: string) {
    return await ConfigApi.removeExtension({ key: name });
  }
}
