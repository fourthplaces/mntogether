interface UrgentNote {
  content: string;
  ctaText?: string | null;
}

/**
 * UrgentBanner — Left-bordered alert banner for urgent notes.
 * Wraps existing urgent-banner CSS with a proper component interface.
 */
export function UrgentBanner({ notes }: { notes: UrgentNote[] }) {
  if (!notes || notes.length === 0) return null;

  return (
    <div className="urgent-banner">
      <div className="urgent-banner__label mono-sm">Urgent</div>
      {notes.map((note, i) => (
        <div key={i}>
          {note.ctaText && <p className="urgent-banner__cta">{note.ctaText}</p>}
          <p>{note.content}</p>
        </div>
      ))}
    </div>
  );
}
