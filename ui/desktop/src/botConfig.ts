/**
 * Bot configuration interface
 */
export interface BotConfig {
  instructions: string;
  activities: string[] | undefined;
  [key: string]: string | string[] | undefined;
}
