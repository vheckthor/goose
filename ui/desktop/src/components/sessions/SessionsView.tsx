import React, { useState, useRef } from 'react';
import { ViewConfig } from '../../App';
import { fetchSessionDetails, type SessionDetails } from '../../sessions';
import { fetchSharedSessionDetails } from '../../sharedSessions';
import { importSessionFromFile } from '../../sessionFiles';
import SessionListView from './SessionListView';
import SessionHistoryView from './SessionHistoryView';
import { Card } from '../ui/card';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import { toast } from 'react-toastify';

interface SessionsViewProps {
  setView: (view: ViewConfig['view'], viewOptions?: Record<any, any>) => void;
}

const SessionsView: React.FC<SessionsViewProps> = ({ setView }) => {
  const [selectedSession, setSelectedSession] = useState<SessionDetails | null>(null);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [shareToken, setShareToken] = useState('6dc69efc-5599-4493-968e-8615a93dd780');
  const [baseUrl, setBaseUrl] = useState(
    'https://goosed-playpen-smohammed--goosed.stage.sqprod.co/api'
  );
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleSelectSession = async (sessionId: string) => {
    await loadSessionDetails(sessionId);
  };

  const loadSessionDetails = async (sessionId: string) => {
    setIsLoadingSession(true);
    setError(null);
    try {
      const sessionDetails = await fetchSessionDetails(sessionId);
      setSelectedSession(sessionDetails);
    } catch (err) {
      console.error(`Failed to load session details for ${sessionId}:`, err);
      setError('Failed to load session details. Please try again later.');
      // Keep the selected session null if there's an error
      setSelectedSession(null);
    } finally {
      setIsLoadingSession(false);
    }
  };

  const handleBackToSessions = () => {
    setSelectedSession(null);
    setError(null);
  };

  const handleResumeSession = () => {
    if (selectedSession) {
      // Get the working directory from the session metadata
      const workingDir = selectedSession.metadata.working_dir;

      if (workingDir) {
        console.log(
          `Resuming session with ID: ${selectedSession.session_id}, in working dir: ${workingDir}`
        );

        // Create a new chat window with the working directory and session ID
        window.electron.createChatWindow(
          undefined,
          workingDir,
          undefined,
          selectedSession.session_id
        );
      } else {
        // Fallback if no working directory is found
        console.error('No working directory found in session metadata');
        // We could show a toast or alert here
      }
    }
  };

  const handleRetryLoadSession = () => {
    if (selectedSession) {
      loadSessionDetails(selectedSession.session_id);
    }
  };

  const handleTestSharedSession = async () => {
    if (!shareToken || !baseUrl) {
      setError('Please enter both a share token and base URL');
      return;
    }

    setIsLoadingSession(true);
    setError(null);

    try {
      // Get the saved base URL from localStorage if available
      let finalBaseUrl = baseUrl;
      if (!finalBaseUrl) {
        const savedSessionConfig = localStorage.getItem('session_sharing_config');
        if (savedSessionConfig) {
          const config = JSON.parse(savedSessionConfig);
          if (config.enabled && config.baseUrl) {
            finalBaseUrl = config.baseUrl;
          }
        }
      }

      if (!finalBaseUrl) {
        throw new Error('Base URL is not configured. Please enter a base URL.');
      }

      // Fetch the shared session details
      const sessionDetails = await fetchSharedSessionDetails(finalBaseUrl, shareToken);

      // Navigate to the shared session view
      setView('sharedSession', {
        sessionDetails,
        shareToken,
        baseUrl: finalBaseUrl,
      });
    } catch (error) {
      console.error('Failed to load shared session:', error);
      setError(
        `Failed to load shared session: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    } finally {
      setIsLoadingSession(false);
    }
  };

  const handleImportSession = () => {
    // Trigger the file input click
    if (fileInputRef.current) {
      fileInputRef.current.click();
    }
  };

  const handleFileSelected = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    setIsLoadingSession(true);
    setError(null);

    try {
      const sessionDetails = await importSessionFromFile(file);
      setSelectedSession(sessionDetails);
      toast.success('Session imported successfully');
    } catch (error) {
      console.error('Failed to import session:', error);
      toast.error(
        `Failed to import session: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
      setError(
        `Failed to import session: ${error instanceof Error ? error.message : 'Unknown error'}`
      );
    } finally {
      setIsLoadingSession(false);
      // Reset the file input
      if (fileInputRef.current) {
        fileInputRef.current.value = '';
      }
    }
  };

  // If a session is selected, show the session history view
  // Otherwise, show the sessions list view with a button to test shared sessions
  return selectedSession ? (
    <SessionHistoryView
      session={selectedSession}
      isLoading={isLoadingSession}
      error={error}
      onBack={handleBackToSessions}
      onResume={handleResumeSession}
      onRetry={handleRetryLoadSession}
    />
  ) : (
    <>
      <SessionListView setView={setView} onSelectSession={handleSelectSession} />

      {/* Hidden file input for importing sessions */}
      <input
        type="file"
        ref={fileInputRef}
        onChange={handleFileSelected}
        accept=".json"
        style={{ display: 'none' }}
      />

      {/* Import button */}
      <div className="fixed bottom-6 right-6">
        <Button onClick={handleImportSession} className="bg-primary hover:bg-primary/90 text-white">
          Import Session
        </Button>
      </div>
    </>
  );
};

export default SessionsView;
