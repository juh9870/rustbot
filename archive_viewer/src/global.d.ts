import { Props } from 'janadom';

type El1 = Element;
declare global {
  declare namespace JSX {
    // The return type of our JSX Factory: this could be anything
    type Element = HTMLElement;

    // IntrinsicElementMap grabs all the standard HTML tags in the TS DOM lib.
    interface IntrinsicElements extends IntrinsicElementMap {}

    // The following are custom types, not part of TS's known JSX namespace:
    type IntrinsicElementMap = {
      [K in keyof HTMLElementTagNameMap | 'svg']: Props;
    };

    interface Component {
      (properties?: { [key: string]: any }, children?: Node[]): Node;
    }
  }

  interface Window {
    jsonData?: string;
  }
}
