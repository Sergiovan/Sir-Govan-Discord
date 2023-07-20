import { Canvas, FontLibrary, CanvasRenderingContext2D, CanvasGradient, Image, Font } from 'skia-canvas';
import { Logger } from '../utils';
import twemoji from 'twemoji'; 

type RGB = [r: number, g: number, b: number];

const FONTS = {
  adobe: FontLibrary.use("adobe-garamond-pro", [
    'media/Adobe*.ttf'
  ])[0],
  optimus: FontLibrary.use("optimus-princeps", [
    'media/Optimus*.ttf'
  ])[0],
  times: FontLibrary.use("times-new-roman", [
    'media/Times*.ttf'
  ])[0],
} satisfies Record<string, Font>;

// Dark souls text taken from https://github.com/Sibert-Aerts/sibert-aerts.github.io/commit/47744dbce5b4665c3500345c50786a33dee964af

export const GRADIENTS = {
  gay:    ['#f00', '#f80', '#fe0', '#0a0', '#26c', '#a0a'],
  trans:  ['#7bf', '#7bf', '#f9a', '#f9a', '#fff', '#fff', '#f9a', '#f9a', '#7bf', '#7bf'],
  bi:     ['#f08', '#f08', '#a6a', '#80f', '#80f'],
  les:    ['#f20', '#f64', '#fa8', '#fff', '#f8f', '#f4c', '#f08'],
  nb:     ['#ff2', '#fff', '#84d', '#333'],
  pan:    ['#f2c', '#f2c', '#ff2', '#ff2', '#2cf', '#2cf'],
  men:    ['#2a6', '#6ea', '#8fc', '#fff', '#88f', '#44c', '#22a'],
};

type GRADIENTS_KEY = keyof typeof GRADIENTS;

// The same colour gradients but formatted as [r,g,b] arrays
let GRADIENTS_RGB: {[key in GRADIENTS_KEY]: RGB[]} = {} as {[key in GRADIENTS_KEY]: RGB[]}; // :)
for (const key in GRADIENTS) {
  const ts_key: GRADIENTS_KEY = key as GRADIENTS_KEY;
  GRADIENTS_RGB[ts_key] = []
  for (const hex of GRADIENTS[ts_key]) {
    GRADIENTS_RGB[ts_key].push([
      parseInt(hex[1], 16) * 17, 
      parseInt(hex[2], 16) * 17, 
      parseInt(hex[3], 16) * 17
    ]);
  }
}

export interface Preset {
  main_color: RGB;
  sheen_tint: RGB;

  text_spacing: number;
  text_opacity?: number;

  sheen_size: number;
  sheen_opacity: number;

  shadow_opacity?: number;

  font?: keyof typeof FONTS;
  font_specific?: string;
  font_weight?: 'bold' | 'bolder' | 'normal' | 'lighter' | `${number}`;
};

export const PRESETS = {
  HUMANITY_RESTORED: {
    main_color: [129, 187, 153],
    sheen_tint: [255, 178, 153],

    text_spacing: 8,
    sheen_size: 1.1,
    sheen_opacity: 0.08,
    font: 'adobe'
  },
  VICTORY_ACHIEVED: {
    main_color: [255, 255, 107],
    sheen_tint: [187, 201, 192],

    text_spacing: 0,
    sheen_size: 1.16,
    sheen_opacity: 0.08,
    font: 'adobe'
  },
  BONFIRE_LIT: {
    main_color: [255, 228, 92],
    sheen_tint: [251, 149, 131],

    text_spacing: 1,
    sheen_size: 1.14,
    sheen_opacity: 0.1,
    font: 'adobe'
  },
  YOU_DIED: {
    main_color: [101, 5, 4],
    sheen_tint: [0, 0, 0],

    text_spacing: 8,
    text_opacity: 1,

    sheen_size: 0,
    sheen_opacity: 0,

    shadow_opacity: 1,
    
    font: 'optimus',
    font_weight: 'bold'
  }
} satisfies Record<string, Preset>;

function multiply_rgb(left: RGB, right: RGB): RGB {
  return [
    left[0] * right[0] / 255, 
    left[1] * right[1] / 255, 
    left[2] * right[2] / 255
  ];
}

function clamp_to_integer(x: number, min: number, max: number): number {
  return Math.max(Math.min(Math.floor(x), max), min);
}

function draw_background(ctx: CanvasRenderingContext2D, canvas: Canvas, preset: Preset, scale: number): void {
  const w = canvas.width, h = canvas.height;

  const shadowSize = 1;
  const shadowOpacity = preset.shadow_opacity ?? 0.7;
  const shadowOffset = 0.0; 
  const shadowSoftness = 1;

  if (shadowSize <= 0) return;

  const shadowHeight = shadowSize * .95 * h * scale;
  const shadowCenter = shadowOffset * scale * h;
  const top = shadowCenter - shadowHeight / 2;
  const bottom = shadowCenter + shadowHeight / 2;

  const softnessLow  = Math.min(1, shadowSoftness);
  const softnessHigh = Math.max(1, shadowSoftness) - 1;

  const gradient = ctx.createLinearGradient(0, top, 0, bottom);
  gradient.addColorStop(0, '#0000');
  gradient.addColorStop(0.25 * softnessLow, `rgba(0, 0, 0, ${shadowOpacity})`);
  gradient.addColorStop(1 - 0.25 * softnessLow, `rgba(0, 0, 0, ${shadowOpacity})`);
  gradient.addColorStop(1, '#0000');
  ctx.fillStyle = gradient;

  if (softnessHigh > 0) {
    ctx.filter = `blur(${Math.floor(shadowHeight * softnessHigh / 4)}px)`;
  }

  ctx.fillRect(-shadowHeight / 2, top, w + shadowHeight, shadowHeight);
  ctx.filter = 'none';
}

type caption = Array<string | Image>[];

async function create_caption_data(ctx: CanvasRenderingContext2D, preset: Preset, scale: number, text: string): Promise<[caption, number] | null> {
  const charSpacing = preset.text_spacing;
  const textColor: RGB = preset.main_color;

  const font = preset.font ? FONTS[preset.font] : {family: preset.font_specific, weight: preset.font_weight};
  const fontSize = 92;
  const fontFamily = preset.font_specific ?? font.family;
  const vScale = 1.5;
  const fontWeight = preset.font_weight ?? font.weight;

  let lines = text.replace(" src=", " scr=").split('\n');
  let res: caption = [];
  
  let promises: Promise<any>[] = [];

  for (let line of lines) {
    let content: Array<string | Image> = [];

    line = twemoji.parse(line, {
      callback: function(icon: string, options: TwemojiOptions) {
          switch ( icon ) {
              case 'a9':      // © copyright
              case 'ae':      // ® registered trademark
              case '2122':    // ™ trademark
                  return false;
          }
          return `${options.base}${options.size}/${icon}${options.ext}`;
      }
    });
    const elems: string[] = line.split(/(\<a?\:.*?\:[0-9]+\>)/g).map((str => str.split(/(\<img.*?src=".*?"\/\>)/))).flat(1);

    for (let part of elems) {
      let match = part.match(/^\<a?\:.*?\:([0-9]+)\>$|\<img.*?src="(.*?)"\/\>/);
      if (match) {
        let src: string;
        let emoji_image: Image = new Image;
        if (match[2]) {
          // Url
          src = match[2];
        } else {
          // Emoji
          src = `https://cdn.discordapp.com/emojis/${match[1]}.png`;
        }
        content.push(emoji_image);
        promises.push(new Promise((res, rej) => { 
          emoji_image.onload = res;
          setTimeout(() => {
            const err = `Could not load image from ${src}`;
            Logger.error(`Could not load image from ${src}`);
            rej(`Could not load image from ${src}`);
          }, 2000);
        }));
        emoji_image.src = src;
      } else {
        // Text
        if (charSpacing > 0) {
          const space = ' '.repeat(Math.floor(charSpacing / 5));
          part = space + part.toUpperCase().split('').join(space) + space;
        } else {
          part = part.toUpperCase();
        }
        content.push(part);
      }
    }
    res.push(content);
  }
  
  try {
    await Promise.all(promises);
  } catch {
    return null;
  }

  ctx.font = `${fontWeight} ${fontSize * scale}px ${fontFamily}`;
  ctx.fillStyle = `rgb(${textColor.join()})`;
  // ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  
  ctx.scale(1, vScale);

  return [res, vScale];
}

function create_gradient(ctx: CanvasRenderingContext2D, colors: RGB[], width: number, x: number = 0, opacity: number = 1): CanvasGradient {
  const gradient = ctx.createLinearGradient(x - width / 2, 0, x + width / 2, 0);
  for (let i = 0; i < colors.length; ++i) {
    const [r, g, b] = colors[i];
    gradient.addColorStop(i / (colors.length - 1), `rgba(${r}, ${g}, ${b}, ${opacity})`);
  }
  return gradient; 
}

function create_default_gradient(ctx: CanvasRenderingContext2D, key: GRADIENTS_KEY, width: number, x: number = 0, opacity: number = 1, mul: RGB | undefined = undefined) {
  let colors = GRADIENTS_RGB[key];
  if (mul) {
    colors = colors.map((c: RGB) => multiply_rgb(c, mul).map(x => clamp_to_integer(x, 0, 255))) as RGB[];
  }
  return create_gradient(ctx, colors, width, x, opacity);
}

const reduce_max = (acc: number, x: number) => Math.max(acc, x);
const reduce_sum = (acc: number, x: number) => acc + x;

function draw_caption(ctx: CanvasRenderingContext2D, canvas: Canvas, lines: caption, y_offset: number, font_size: number, vertical_scale: number) {

  let heights: number[][] = [], widths: number[][] = [];
  for (let line of lines) {
    let height: number[] = [];
    heights.push(height);
    let width: number[] = [];
    widths.push(width);
    for (let part of line) {
      if (part instanceof Image) {
        if (part.src.includes('twemoji')) {
          width.push(128);
          height.push(128 * (1 / vertical_scale));
        } else {
          width.push(part.width);
          height.push(part.height * (1 / vertical_scale));
        }
      } else {
        const measurements = ctx.measureText(part);
        width.push(measurements.width);
        height.push(measurements.actualBoundingBoxDescent + measurements.actualBoundingBoxAscent);
      }
    }
  }

  const line_heights = heights.map((heights) => heights.reduce(reduce_max, 0));
  const total_height = line_heights.reduce(reduce_sum, 0);
  const average_line_height = total_height / lines.length;

  const line_widths = widths.map((widths) => widths.reduce(reduce_sum, 0));
  const total_width = line_widths.reduce(reduce_max, 0) + 100;

  if (total_width > canvas.width) {
    ctx.scale(canvas.width / total_width, canvas.width / total_width);
  }

  ctx.save();

  for (let i = 0; i < lines.length; ++i) {
    const total_width = widths[i].reduce(reduce_sum, 0);
    let x = - (total_width / 2);
    let x0 = x;
    for (let j = 0; j < lines[i].length; ++j){
      const part = lines[i][j];
      if (part instanceof Image) {
        // Emoji
        ctx.drawImage(part, x, (y_offset + average_line_height * i) - ((lines.length - 1) * (average_line_height / 2)) - (line_heights[i] / 2) + ((line_heights[i] - heights[i][j]) / 2), widths[i][j], heights[i][j]);
        x += widths[i][j];
      } else {
        // Text
        ctx.fillText(part, x, (y_offset + average_line_height * i) - ((lines.length - 1) * (average_line_height / 2)));
        x += widths[i][j];
      }
    }
    x = x0;
  }

  ctx.restore();
}

function get_size_heuristics(text: string, preset: Preset): number {
  const canvas = new Canvas(0, 0);
  const ctx = canvas.getContext('2d');

  const font = preset.font ? FONTS[preset.font] : {family: preset.font_specific, weight: preset.font_weight};
  const fontSize = 92;
  const fontFamily = preset.font_specific ?? font.family;
  const fontWeight = preset.font_weight ?? font.weight;

  ctx.font = `${fontWeight} ${fontSize}px ${fontFamily}`;
  ctx.textBaseline = 'middle';

  return clamp_to_integer(text.split('\n').map(l => ctx.measureText(l).width).reduce(reduce_max) + 100, 1200, 1920);
}

export async function create_dark_souls_image(text: string, preset: Preset, gradient_key: GRADIENTS_KEY | "" = "") {

  if (!text) return null;

  const w = get_size_heuristics(text, preset), h = 280;
  const canvas = new Canvas(w, h);
  const ctx = canvas.getContext('2d');

  // CONSTANTS
  let s = h / 280;

  // USER INPUT
  const xOffset = 0;
  const yOffset = 0;
  const scale = 1;

  const textOpacity = preset.text_opacity ?? 0.9; 
  const blurTint: RGB = preset.sheen_tint;
  const blurSize = preset.sheen_size; 
  const blurOpacity = preset.sheen_opacity;
  
  const textColor: RGB = preset.main_color;
  const fontSize = 92;
  
  const gradient: GRADIENTS_KEY | "" = typeof preset === 'string' ? preset : "";
  const gradientScale = 0.5;

  const x0 = xOffset * w + w / 2;
  const y0 = yOffset * h + h / 2;
  s *= scale;

  //// SHADE
  // The shade only moves up or down
  ctx.translate(0, y0);
  draw_background(ctx, canvas, preset, scale);
  ctx.translate(x0, 0);

  //// TEXT
  const caption_data = await create_caption_data(ctx, preset, s, text);
  if (!caption_data) return null;
  const [lines, vScale] = caption_data;
  ctx.save();

  //// Emulate the zoom blur effect
  const zoomSteps = Math.floor(20 * blurSize * Math.pow(s, 1/4)) || -1;
  // Zoom blur vertical distance
  const VOFFSET = 1;
  const voff = VOFFSET * s / (blurSize - 1);

  if (gradient) {
    ctx.fillStyle = create_default_gradient(ctx, gradient, gradientScale * w * s, 0, 1, blurTint);
  } else {
    ctx.fillStyle = `rgb(${multiply_rgb(textColor, blurTint).map((x) => clamp_to_integer(x, 0, 255))})`;
  }

  // Draw the zoom blur as a bunch of layers, back-to-front for proper effect
  for (let i = zoomSteps; i >= 0; --i) {
      ctx.save();

      // `scaleFactor` ranges from 1 up to and including blurSize
      const scaleFactor = Math.pow(blurSize, i / zoomSteps);
      if (i) {
        ctx.scale(scaleFactor, scaleFactor);
      }

      // `fatProduct` ranges from 1 up to and including approx. 2
      const fatProduct = Math.pow(scaleFactor, 1 / Math.log2(blurSize));

      ctx.filter = `blur(${Math.floor(s * scaleFactor ** 4)}px)`;
      ctx.globalAlpha = blurOpacity / fatProduct;
      draw_caption(ctx, canvas, lines, voff * (scaleFactor - 1) / vScale, fontSize * s * 0.9, vScale);
      ctx.restore();
  }
  
  ctx.restore();

  // Draw the regular text on top
  if (gradient) {
      ctx.fillStyle = create_default_gradient(ctx, gradient, gradientScale * w * s, 0, textOpacity);
  } else {
      ctx.fillStyle = `rgba(${textColor}, ${textOpacity})`;
  }

  draw_caption(ctx, canvas, lines, 0, fontSize * s * 0.9, vScale);

  return canvas.toBuffer('png');
}