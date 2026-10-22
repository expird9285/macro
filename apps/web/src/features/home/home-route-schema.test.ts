import { describe, expect, it } from 'vitest';
import { homePreviewRouteParams } from './home-route-schema';

describe('home preview route params', () => {
  it('opens a PDF item in the pdf block', () => {
    expect(
      homePreviewRouteParams.safeParse({ blockType: 'pdf', previewId: 'doc' })
        .success
    ).toBe(true);
  });

  it('leaves calendar to the calendar route', () => {
    expect(
      homePreviewRouteParams.safeParse({
        blockType: 'calendar',
        previewId: 'id',
      }).success
    ).toBe(false);
  });
});
