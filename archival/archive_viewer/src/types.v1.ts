export type Root = Message[];

export interface Message {
  activity: unknown;
  application: unknown;
  application_id?: string;
  attachments: Attachment[];
  author: Author;
  channel_id: string;
  components: unknown[];
  content: string;
  edited_timestamp?: string;
  embeds: Embed[];
  flags: number;
  guild_id: unknown;
  id: string;
  interaction?: Interaction;
  member: unknown;
  mention_channels: unknown[];
  mention_everyone: boolean;
  mention_roles: string[];
  mentions: Mention[];
  message_reference?: MessageReference;
  nonce: unknown;
  pinned: boolean;
  reactions: Reaction[];
  'mention_channels::processed': Record<string, string>;
  'stickers::processed': Record<string, string>;
  'mention_roles::processed': MentionRolesProcessed[];
  'author_avatar::processed'?: string;
  'reactions::processed': ReactionsProcessed[];
  referenced_message?: ReferencedMessage;
  sticker_items: StickerItem[];
  thread: unknown;
  timestamp: string;
  tts: boolean;
  type: number;
  webhook_id?: string;
}

export interface MentionRolesProcessed {
  color: number;
  guild_id: string;
  hoist: boolean;
  icon: unknown;
  id: string;
  managed: boolean;
  mentionable: boolean;
  name: string;
  permissions: string;
  position: number;
  tags: Tags;
  unicode_emoji: unknown;
}

export interface Tags {
  bot_id: unknown;
  integration_id: unknown;
}

export interface Attachment {
  content_type?: string;
  filename: string;
  height?: number;
  id: string;
  proxy_url: string;
  size: number;
  url: string;
  width?: number;
}

export interface Author {
  accent_color: unknown;
  avatar?: string;
  banner: unknown;
  bot: boolean;
  discriminator: string;
  id: string;
  member: unknown;
  public_flags: number;
  username: string;
}

export interface Embed {
  author?: EmbedAuthor;
  color?: number;
  description?: string;
  fields: Field[];
  footer: unknown;
  image?: Image;
  provider?: Provider;
  thumbnail?: Thumbnail;
  timestamp: unknown;
  title?: string;
  type: string;
  url?: string;
  video?: Video;
}

export interface EmbedAuthor {
  icon_url: unknown;
  name: string;
  proxy_icon_url: unknown;
  url?: string;
}

export interface Field {
  inline: boolean;
  name: string;
  value: string;
}

export interface Image {
  height: number;
  proxy_url: string;
  url: string;
  width: number;
}

export interface Provider {
  name: string;
  url?: string;
}

export interface Thumbnail {
  height: number;
  proxy_url: string;
  url: string;
  width: number;
}

export interface Video {
  height: number;
  proxy_url?: string;
  url: string;
  width: number;
}

export interface Interaction {
  id: string;
  name: string;
  type: number;
  user: User;
}

export interface User {
  accent_color: unknown;
  avatar: string;
  banner: unknown;
  bot: boolean;
  discriminator: string;
  id: string;
  member: unknown;
  public_flags: number;
  username: string;
}

export interface Mention {
  accent_color: unknown;
  avatar?: string;
  banner: unknown;
  bot: boolean;
  discriminator: string;
  id: string;
  member: unknown;
  public_flags: number;
  username: string;
}

export interface MessageReference {
  channel_id: string;
  guild_id?: string;
  message_id: string;
}

export interface Reaction {
  count: number;
  emoji: Emoji;
  me: boolean;
}

export interface Emoji {
  animated?: boolean;
  id?: string;
  name: string;
}

export interface ReactionsProcessed {
  count: number;
  path: string;
}

export interface ReferencedMessage {
  activity: unknown;
  application: unknown;
  application_id: unknown;
  attachments: Attachment[];
  author: Author;
  channel_id: string;
  components: unknown[];
  content: string;
  edited_timestamp?: string;
  embeds: Embed[];
  flags: number;
  guild_id: unknown;
  id: string;
  interaction: unknown;
  member: unknown;
  mention_channels: unknown[];
  mention_everyone: boolean;
  mention_roles: unknown[];
  mentions: Mention[];
  message_reference?: MessageReference;
  nonce: unknown;
  pinned: boolean;
  reactions: unknown[];
  referenced_message: unknown;
  sticker_items: StickerItem[];
  thread: unknown;
  timestamp: string;
  tts: boolean;
  type: number;
  webhook_id: unknown;
}

export interface StickerItem {
  format_type: number;
  id: string;
  name: string;
}
