import { Attachment, Message } from './types.v1';
import { createEl } from 'janadom';
import './reset.css';
import './index.css';
import './markdown.css';
// @ts-ignore
import prettyBytes from 'pretty-bytes';
import { Marked } from '@ts-stack/markdown';
// import * as highlighter from 'highlight.js';
import 'highlight.js/styles/github-dark.css';

const file_pic = `data:image/svg+xml;base64,PHN2ZyBmaWxsPSJub25lIiBoZWlnaHQ9Ijk2IiB2aWV3Qm94PSIwIDAgNzIgOTYiIHdpZHRoPSI3MiIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cGF0aCBkPSJtNzIgMjkuM3Y2MC4zYzAgMi4yNCAwIDMuMzYtLjQ0IDQuMjItLjM4Ljc0LTEgMS4zNi0xLjc0IDEuNzQtLjg2LjQ0LTEuOTguNDQtNC4yMi40NGgtNTkuMmMtMi4yNCAwLTMuMzYgMC00LjIyLS40NC0uNzQtLjM4LTEuMzYtMS0xLjc0LTEuNzQtLjQ0LS44Ni0uNDQtMS45OC0uNDQtNC4yMnYtODMuMmMwLTIuMjQgMC0zLjM2LjQ0LTQuMjIuMzgtLjc0IDEtMS4zNiAxLjc0LTEuNzQuODYtLjQ0IDEuOTgtLjQ0IDQuMjItLjQ0aDM2LjNjMS45NiAwIDIuOTQgMCAzLjg2LjIyLjUuMTIuOTguMjggMS40NC41djE2Ljg4YzAgMi4yNCAwIDMuMzYuNDQgNC4yMi4zOC43NCAxIDEuMzYgMS43NCAxLjc0Ljg2LjQ0IDEuOTguNDQgNC4yMi40NGgxNi44OGMuMjIuNDYuMzguOTQuNSAxLjQ0LjIyLjkyLjIyIDEuOS4yMiAzLjg2eiIgZmlsbD0iI2QzZDZmZCIvPjxwYXRoIGQ9Im02OC4yNiAyMC4yNmMxLjM4IDEuMzggMi4wNiAyLjA2IDIuNTYgMi44OC4xOC4yOC4zMi41Ni40Ni44NmgtMTYuODhjLTIuMjQgMC0zLjM2IDAtNC4yMi0uNDQtLjc0LS4zOC0xLjM2LTEtMS43NC0xLjc0LS40NC0uODYtLjQ0LTEuOTgtLjQ0LTQuMjJ2LTE2Ljg4MDAyOWMuMy4xNC41OC4yOC44Ni40NTk5OTkuODIuNSAxLjUgMS4xOCAyLjg4IDIuNTZ6IiBmaWxsPSIjOTM5YmY5Ii8+PHBhdGggY2xpcC1ydWxlPSJldmVub2RkIiBkPSJtMjQgMjRjNC40MiAwIDgtMy41OCA4LTggMC0uNzItLjEtMS40Mi0uMjgtMi4wOGwtMy43Mi0xMy45MmgtOGwtMy43MiAxMy45MmMtLjE4LjY2LS4yOCAxLjM2LS4yOCAyLjA4IDAgNC40MiAzLjU4IDggOCA4em0wLTRjMi4yMDkxIDAgNC0xLjc5MDkgNC00cy0xLjc5MDktNC00LTQtNCAxLjc5MDktNCA0IDEuNzkwOSA0IDQgNHptMCAyMHYtOGgtOHY4em0wIDhoOHYtOGgtOHptMCA4di04aC04djh6bTAgOGg4di04aC04em0wIDh2LThoLTh2OHptMCA4aDh2LThoLTh6bTAgOGgtOHYtOGg4em0wIDBoOHY4aC04eiIgZmlsbD0iIzU4NjVmMiIgZmlsbC1ydWxlPSJldmVub2RkIi8+PC9zdmc+`;

Marked.setOptions({
  tables: false,
  breaks: true,
  // highlight: (code, lang) => {
  //   try {
  //     return highlighter.default.highlight(lang!, code).value;
  //   } catch {
  //     return code;
  //   }
  // },
});

function parseContent(content: string, message: Message): HTMLElement[] {
  let element = <div></div>;
  let parsed = Marked.parse(content);
  // console.log(parsed);

  parsed = parsed.replace(/&lt;:\w+:(\d{18})&gt;/g, (_, id) => {
    return `<img class="inline-emoji" src="assets/${id}.png" alt="${id}"></img>`;
  });
  parsed = parsed.replace(/<t:(\d+)(?::.)?>/g, (_, time) => {
    let date = new Date(+time * 1000);
    return `<span class="mention timestamp">${date.toUTCString()}</span>`;
  });
  parsed = parsed.replace(/&lt;@!?(\d{18})&gt;/g, (text, id) => {
    let mentioned = message.mentions.find((e) => e.id === id);
    if (!mentioned) return text;
    return `<span class="mention">@${mentioned.username}</span>`;
  });

  parsed = parsed.replace(/&lt;@&amp;(\d{18})&gt;/g, (text, id) => {
    // console.log(text, id);
    let mentioned = message['mention_roles::processed'].find(
      (e) => e.id === id,
    );
    if (!mentioned) return text;
    let color = '#' + mentioned.color.toString(16).padStart(6, '0');
    return `<span class="mention" style="color: ${color}; background-color: ${color}1a">@${mentioned.name}</span>`;
  });

  parsed = parsed.replace(/&lt;#(\d{18})&gt;/g, (text, id) => {
    // console.log(text, id);
    let mentioned = message['mention_channels::processed'][id];
    if (!mentioned) return text;
    return `<span class="mention">#${mentioned}</span>`;
  });

  element.innerHTML = parsed;

  return [...(element.children as unknown as HTMLElement[])];
}

function attachment(attachment: Attachment): HTMLElement {
  let extension = attachment.url.toLowerCase().split('.').pop()!;
  if (
    attachment.content_type?.startsWith('image') ||
    extension.match(/png|jpg|jpeg|jfif|pjpeg|pjp|svg|gif|webp|apng|avif/)
  ) {
    return (
      <img src={attachment.url} alt={attachment.filename} class='image'></img>
    );
  } else {
    return (
      <a class='attachment flex-row' href={attachment.url}>
        <img src={file_pic} alt='file icon' />
        <div class='attachment-body'>
          <div class='attachment-title'>{attachment.filename}</div>
          <div class='attachment-size'>{prettyBytes(attachment.size)}</div>
        </div>
      </a>
    );
  }
}

function highlight(targetId: string) {
  let element = document.getElementById(targetId);
  if (element) {
    element.classList.remove('highlight');
    setTimeout(() => {
      element!.classList.add('highlight');
    }, 0);
  }
}

function message(
  message: Message,
  previous: Message | undefined,
  old: Record<string, Message>,
): HTMLElement {
  let date = new Date(message.timestamp);
  let short_time = date.toTimeString().slice(0, 5);
  let show_header =
    previous?.author.id != message.author.id || message.referenced_message;
  let replyTo = old[message.referenced_message?.id ?? ''];
  let reply: HTMLElement | null = null;
  if (replyTo) {
    let content =
      replyTo.content.length === 0 ? (
        <div class='reply-text reply-text-attachment'>
          Click to see attachment
        </div>
      ) : (
        <div class='reply-text'>{parseContent(replyTo.content, replyTo)}</div>
      );
    reply = (
      <a
        class='flex-row reply'
        href={'#' + replyTo.id}
        onclick={() => highlight(replyTo!.id)}
      >
        <div class='reply-spacer'>
          <div class='reply-spacer-inner'></div>
        </div>
        <div class='flex-row header-reply'>
          <img
            src={message.author.avatar}
            alt={message.author.username}
            class='pfp-reply'
          ></img>
          <div class='reply-username'>{replyTo.author.username}</div>
          {content}
        </div>
      </a>
    );
  }
  return (
    <div class='message' id={message.id}>
      {reply}
      <div class='flex-row'>
        {show_header ? (
          <img
            src={message.author.avatar}
            alt={message.author.username}
            class='pfp'
          ></img>
        ) : (
          <div class='pfp-spacer time'>{short_time}</div>
        )}
        <div class='body'>
          {show_header ? (
            <span class='header'>
              <span class='username'>{message.author.username}</span>{' '}
              <span class='time'>{date.toUTCString()}</span>
            </span>
          ) : null}
          <div class='content markdown'>
            {parseContent(message.content, message)}
          </div>
          {message.attachments.map(attachment)}
        </div>
      </div>
    </div>
  );
}

function showJson(data: unknown) {
  let messages = data as Message[];
  let old: Record<string, Message> = {};
  let processed_messages: HTMLElement[] = [];
  for (let i = messages.length - 1; i >= 0; i--) {
    let current = messages[i]!;
    let previous = messages[i + 1];
    processed_messages.push(message(current, previous, old));
    old[current.id] = current;
  }
  document.body.appendChild(<div>{processed_messages}</div>);
}

if (!window.jsonData) {
  alert('messages.jsonp is not found!');
  throw new Error('messages.jsonp is not found!');
} else {
  showJson(window.jsonData);
}
