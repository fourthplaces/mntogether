/**
 * PhotoBreak — full-width editorial photo used as a visual break between
 * content sections. Same newsprint-photo treatment as FeaturePhoto but
 * rendered from a widget record (not a post).
 */

interface PhotoBreakProps {
  image: string;
  caption?: string;
  credit?: string;
}

export function PhotoBreak({ image, caption, credit }: PhotoBreakProps) {
  if (!image) return null;
  return (
    <div className="feat-photo" data-debug="Widget.photo">
      <div
        className="feat-photo__image newsprint-photo"
        style={{ backgroundImage: `url('${image}')` }}
      />
      {(caption || credit) && (
        <div className="feat-photo__caption">
          {caption && <div className="feat-photo__text">{caption}</div>}
          {credit && <div className="feat-photo__credit">{credit}</div>}
        </div>
      )}
    </div>
  );
}
