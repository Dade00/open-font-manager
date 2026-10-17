import { useEffect, useRef } from 'react';

export function InspirationConfirm({ title, onConfirm, onCancel }: { title: string; onConfirm(): void; onCancel(): void }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current;
    const previous = document.activeElement as HTMLElement | null;
    if (dialog && !dialog.open) {
      if (typeof dialog.showModal === 'function') dialog.showModal();
      else dialog.setAttribute('open', '');
    }
    return () => {
      if (dialog?.open && typeof dialog.close === 'function') dialog.close();
      previous?.focus();
    };
  }, []);
  const discard = title === 'Scartare modifiche?';
  return <dialog ref={ref} className="modal inspiration-modal" aria-labelledby="inspiration-confirm-title" onCancel={(event) => { event.preventDefault(); onCancel(); }}>
    <div className="modal-box">
      <h2 id="inspiration-confirm-title">{title}</h2>
      <p>{discard ? 'Le modifiche dell’editor non salvate verranno scartate.' : 'La composizione e i suoi riferimenti nelle board verranno rimossi. Font e file originali resteranno invariati.'}</p>
      <div className="modal-action">
        <button className="btn btn-ghost" onClick={onCancel} type="button">Annulla</button>
        <button className="btn btn-error" onClick={onConfirm} type="button">{discard ? 'Scarta modifiche' : 'Elimina e rimuovi riferimenti'}</button>
      </div>
    </div>
  </dialog>;
}
