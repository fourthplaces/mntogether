import type { PhotoData } from '@/lib/broadsheet/detail-types';

/* eslint-disable @next/next/no-img-element */

export function PhotoA({ photo }: { photo: PhotoData }) {
  return (
    <figure className="photo-a">
      <img src={photo.src} alt={photo.alt || ''} loading="lazy" />
      <figcaption className="photo-a__caption">
        <span className="photo-a__caption-text">{photo.caption}</span>
        <span className="photo-a__credit mono-sm">{photo.credit}</span>
      </figcaption>
    </figure>
  );
}

export function PhotoB({ photo }: { photo: PhotoData }) {
  return (
    <figure className="photo-b">
      <img src={photo.src} alt={photo.alt || ''} loading="lazy" />
      <figcaption className="photo-b__caption-bar">
        {photo.caption}<span className="photo-b__credit">{photo.credit}</span>
      </figcaption>
    </figure>
  );
}

export function PhotoC({ photo }: { photo: PhotoData }) {
  return (
    <figure className="photo-c">
      <img src={photo.src} alt={photo.alt || ''} loading="lazy" />
      <figcaption className="photo-c__caption">
        <span className="photo-c__caption-text">{photo.caption}</span>
        <span className="photo-c__credit mono-sm">{photo.credit}</span>
      </figcaption>
    </figure>
  );
}

export function PhotoD({ photo }: { photo: PhotoData }) {
  return (
    <figure className="photo-d">
      <img src={photo.src} alt={photo.alt || ''} loading="lazy" />
      <figcaption className="photo-d__caption">
        <span className="photo-d__caption-text">{photo.caption}</span>
        <span className="photo-d__credit mono-sm">{photo.credit}</span>
      </figcaption>
    </figure>
  );
}
