import { FileSidePanelSections, SidePanel } from '@components/app/side-panel';
import { useSplitLayout } from '@components/app/split-layout/layout';
import { useBlockId } from '@core/block';
import { DocumentBlockContainer } from '@core/component/DocumentBlockContainer';
import { toast } from '@core/component/Toast/Toast';
import { useShareModal } from '@core/component/TopBar/shareModal';
import { fileTypeToBlockName } from '@core/constant/allBlocks';
import { blockMetadataSignal } from '@core/signal/load';
import { useGetPermissions } from '@core/signal/permissions';
import {
  useBlockDocumentDownloadName,
  useBlockDocumentName,
} from '@core/util/currentBlockDocumentName';
import { fireAndForget } from '@core/util/fireAndForget';
import { downloadFile } from '@filesystem/download';
import { waitForDocumentContentReady } from '@queries/storage/document-location';
import { fetchDocumentMetadata } from '@queries/storage/document-metadata';
import { createCallback } from '@solid-primitives/rootless';
import { createSignal, onCleanup, onMount } from 'solid-js';
import {
  legacyOfficeUpgradeTarget,
  pollForLegacyUpgrade,
  shouldAwaitLegacyUpgrade,
  upgradedOfficeLabel,
} from '../legacy-office';
import { useGetFileBlob } from '../signal/blockData';
import { TopBar } from './TopBar';
import { UnknownContent } from './UnknownContent';

export default function BlockUnknown() {
  return (
    <DocumentBlockContainer>
      <div class="size-full select-none overscroll-none overflow-hidden flex flex-col relative">
        <BlockUnknownContent />
      </div>
    </DocumentBlockContainer>
  );
}

function BlockUnknownContent() {
  const fileName = useBlockDocumentName();
  const downloadName = useBlockDocumentDownloadName();
  const blockId = useBlockId();
  const permissions = useGetPermissions();
  const openShare = useShareModal(() => ({
    id: blockId,
    blockAlias: 'unknown',
    itemType: 'document',
    name: fileName() ?? '',
    userPermissions: permissions(),
    owner: blockMetadataSignal()?.owner,
  }));
  const getBlob = useGetFileBlob();
  const convertingTo = useLegacyOfficeUpgrade(blockId);

  const downloadDocument = createCallback(async () => {
    try {
      const blob = await getBlob();
      await downloadFile(blob, downloadName());
    } catch (error) {
      console.error('error downloading file', error);
      toast.failure('Error downloading file');
    }
  });

  return (
    <SidePanel.Layout defaultOpen floating>
      <FileSidePanelSections />
      <div class="flex size-full min-w-0 flex-col overflow-hidden">
        <div class="relative">
          <TopBar onShare={openShare} />
        </div>
        <div class="w-full grow relative overflow-hidden">
          <UnknownContent
            fileName={fileName()}
            convertingTo={convertingTo()}
            onShare={openShare}
            onDownload={() => fireAndForget(downloadDocument(), 'unknown-file')}
          />
        </div>
      </div>
    </SidePanel.Layout>
  );
}

/**
 * Waits for a just-uploaded .doc/.ppt/.xls to be upgraded to OpenXML, then
 * reopens it in the block for its new type. Returns the label of the format
 * being converted to while waiting.
 */
function useLegacyOfficeUpgrade(documentId: string) {
  const { replaceSplit } = useSplitLayout();
  const initial = blockMetadataSignal.get();
  const target = legacyOfficeUpgradeTarget(initial?.fileType);
  const [waiting, setWaiting] = createSignal(
    !!initial && !!target && shouldAwaitLegacyUpgrade(initial, Date.now())
  );

  onMount(() => {
    if (!target || !waiting()) return;
    const controller = new AbortController();
    onCleanup(() => controller.abort());

    fireAndForget(
      (async () => {
        const upgraded = await pollForLegacyUpgrade({
          target,
          fetchMetadata: () => fetchDocumentMetadata(documentId),
          fileTypeOf: (metadata) => metadata.fileType,
          sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
          signal: controller.signal,
        });
        if (controller.signal.aborted) return;
        if (!upgraded) {
          setWaiting(false);
          return;
        }
        // Word content becomes readable once the DOCX pipeline processes it.
        if (target === 'docx') {
          await waitForDocumentContentReady({
            documentId,
            timeoutMs: 60_000,
          }).catch(() => undefined);
          if (controller.signal.aborted) return;
        }
        const blockName = fileTypeToBlockName(upgraded.fileType);
        if (blockName === 'unknown') {
          // Types without a viewer of their own stay in this block.
          blockMetadataSignal.set(upgraded);
          setWaiting(false);
          return;
        }
        replaceSplit({ content: { type: blockName, id: documentId } });
      })(),
      'legacy-office-upgrade'
    );
  });

  return () => (waiting() && target ? upgradedOfficeLabel(target) : undefined);
}
